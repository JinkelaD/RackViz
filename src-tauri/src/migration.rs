use rusqlite::Connection;

const CURRENT_VERSION: i32 = 5;

pub fn run(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap_or(0);

    match version {
        0 => {
            with_migration_tx(conn, migrate_v0_to_v1)?;
            with_migration_tx(conn, migrate_v1_to_v2)?;
            with_migration_tx(conn, migrate_v2_to_v3)?;
            with_migration_tx(conn, migrate_v3_to_v4)?;
            with_migration_tx(conn, migrate_v4_to_v5)?;
        }
        1 => {
            with_migration_tx(conn, migrate_v1_to_v2)?;
            with_migration_tx(conn, migrate_v2_to_v3)?;
            with_migration_tx(conn, migrate_v3_to_v4)?;
            with_migration_tx(conn, migrate_v4_to_v5)?;
        }
        2 => {
            with_migration_tx(conn, migrate_v2_to_v3)?;
            with_migration_tx(conn, migrate_v3_to_v4)?;
            with_migration_tx(conn, migrate_v4_to_v5)?;
        }
        3 => {
            with_migration_tx(conn, migrate_v3_to_v4)?;
            with_migration_tx(conn, migrate_v4_to_v5)?;
        }
        4 => {
            with_migration_tx(conn, migrate_v4_to_v5)?;
        }
        _ => {
            log::warn!("数据库版本 {} 高于当前版本 {}", version, CURRENT_VERSION);
        }
    }
    Ok(())
}

/// 迁移事务保护（审查红线 D-1 / M-4）：每一步迁移在事务中执行，
/// 任一步失败自动回滚，避免留下半截 schema。
fn with_migration_tx<F>(conn: &Connection, f: F) -> Result<(), rusqlite::Error>
where
    F: FnOnce(&Connection) -> Result<(), rusqlite::Error>,
{
    conn.execute_batch("BEGIN IMMEDIATE")?;
    match f(conn) {
        Ok(()) => conn.execute_batch("COMMIT"),
        Err(e) => {
            if let Err(rb_err) = conn.execute_batch("ROLLBACK") {
                log::error!("迁移回滚失败: {}", rb_err);
            }
            Err(e)
        }
    }
}

fn migrate_v0_to_v1(conn: &Connection) -> Result<(), rusqlite::Error> {
    let has_data: bool = conn
        .query_row("SELECT COUNT(*) > 0 FROM rooms", [], |r| r.get(0))
        .unwrap_or(false);

    if has_data {
        conn.execute_batch(
            "UPDATE racks SET room_id = NULL WHERE room_id IS NOT NULL AND room_id NOT IN (SELECT id FROM rooms);
             UPDATE devices SET rack_id = NULL WHERE rack_id IS NOT NULL AND rack_id NOT IN (SELECT id FROM racks);
             UPDATE devices SET device_model_id = NULL WHERE device_model_id IS NOT NULL AND device_model_id NOT IN (SELECT id FROM device_models);"
        )?;
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rooms (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            location    TEXT DEFAULT '',
            sort_order  INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS racks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            height_u    INTEGER NOT NULL DEFAULT 42,
            row         INTEGER DEFAULT 0,
            col         INTEGER DEFAULT 0,
            view        TEXT DEFAULT 'front',
            sort_order  INTEGER DEFAULT 0,
            room_id     INTEGER REFERENCES rooms(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS device_models (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            manufacturer TEXT DEFAULT '',
            type        TEXT NOT NULL DEFAULT 'server',
            height_u    INTEGER NOT NULL DEFAULT 1,
            power_watt  INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS devices (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            device_model_id INTEGER REFERENCES device_models(id) ON DELETE SET NULL,
            rack_id         INTEGER REFERENCES racks(id) ON DELETE SET NULL,
            start_u         INTEGER,
            end_u           INTEGER,
            ip_addresses    TEXT DEFAULT '',
            serial_no       TEXT DEFAULT '',
            asset_no        TEXT DEFAULT '',
            department      TEXT DEFAULT '',
            owner           TEXT DEFAULT '',
            function        TEXT DEFAULT '',
            purchase_date   DATE,
            warranty_expire DATE,
            status          TEXT DEFAULT 'unconfigured',
            power_watt      INTEGER DEFAULT 0
        );

        PRAGMA user_version = 1;"
    )?;

    Ok(())
}

fn migrate_v1_to_v2(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key     TEXT PRIMARY KEY,
            value   TEXT NOT NULL
        );
        INSERT OR IGNORE INTO settings (key, value) VALUES ('logging_enabled', 'false');
        PRAGMA user_version = 2;"
    )?;
    Ok(())
}

/// v2 → v3：添加 7 个索引 + 4 个 UNIQUE 约束（审查红线 P-1 / M-2 / M-3 / D-03）。
/// 索引覆盖全部高频 WHERE / JOIN / 排序查询；唯一约束仅约束非空值，
/// 旧库中已存在的重复 serial/asset 先清理再建索引，重复机房/型号名则跳过并告警。
fn migrate_v2_to_v3(conn: &Connection) -> Result<(), rusqlite::Error> {
    // ---- 1. 清理 devices.serial_no / asset_no 中的历史重复（保留最早一条）----
    conn.execute_batch(
        "UPDATE devices SET serial_no = ''
         WHERE serial_no <> '' AND id NOT IN (
             SELECT MIN(id) FROM devices WHERE serial_no <> '' GROUP BY serial_no
         );

         UPDATE devices SET asset_no = ''
         WHERE asset_no <> '' AND id NOT IN (
             SELECT MIN(id) FROM devices WHERE asset_no <> '' GROUP BY asset_no
         );"
    )?;

    // ---- 2. 普通索引（高频 WHERE / 外键 / 排序）----
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_devices_rack_id ON devices(rack_id);
         CREATE INDEX IF NOT EXISTS idx_devices_device_model_id ON devices(device_model_id);
         CREATE INDEX IF NOT EXISTS idx_devices_rack_name ON devices(rack_id, name);
         CREATE INDEX IF NOT EXISTS idx_racks_room_id ON racks(room_id);
         CREATE INDEX IF NOT EXISTS idx_racks_sort_order ON racks(sort_order);
         CREATE INDEX IF NOT EXISTS idx_rooms_sort_order ON rooms(sort_order);
         CREATE INDEX IF NOT EXISTS idx_device_models_name ON device_models(name);"
    )?;

    // ---- 3. UNIQUE 约束（仅约束非空值，空串/空值不受限）----
    conn.execute_batch(
        "CREATE UNIQUE INDEX IF NOT EXISTS uniq_devices_serial_no
            ON devices(serial_no) WHERE serial_no <> '';
         CREATE UNIQUE INDEX IF NOT EXISTS uniq_devices_asset_no
            ON devices(asset_no) WHERE asset_no <> '';"
    )?;

    // 机房/型号名唯一：若旧库存在重复名称则跳过约束（避免迁移失败），仅告警
    let dup_rooms: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT name FROM rooms WHERE name <> '' GROUP BY name HAVING COUNT(*) > 1)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if dup_rooms > 0 {
        log::warn!("迁移 v3: 检测到 {} 个重复机房名，跳过 rooms.name 唯一约束（请手工整理）", dup_rooms);
    } else {
        conn.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS uniq_rooms_name ON rooms(name) WHERE name <> '';")?;
    }

    let dup_models: i32 = conn
        .query_row(
            "SELECT COUNT(*) FROM (SELECT name FROM device_models WHERE name <> '' GROUP BY name HAVING COUNT(*) > 1)",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    if dup_models > 0 {
        log::warn!("迁移 v3: 检测到 {} 个重复型号名，跳过 device_models.name 唯一约束（请手工整理）", dup_models);
    } else {
        conn.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS uniq_device_models_name ON device_models(name) WHERE name <> '';")?;
    }

    conn.execute_batch("PRAGMA user_version = 3;")?;
    Ok(())
}

/// v3 → v4：设备增加持久"固有高度"列 `devices.height_u`（审查回归：多U设备下架后重上架变 1U）。
/// 此前设备占用高度只隐式存在于 start_u..end_u；下架清空 U 位后高度丢失，
/// 重上架回退到型号高度（无型号则 1U）。v4 把高度落为设备自身字段：
/// - 已在架（start/end 有效区间）→ 回填为实际占用区间高度；
/// - 未上架但有型号 → 回填为型号高度；
/// - 其余保持默认 1。
fn migrate_v3_to_v4(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "ALTER TABLE devices ADD COLUMN height_u INTEGER NOT NULL DEFAULT 1;

         -- 已在架设备：按实际占用区间回填（end >= start 才有效）
         UPDATE devices SET height_u = end_u - start_u + 1
          WHERE start_u IS NOT NULL AND end_u IS NOT NULL AND end_u >= start_u;

         -- 未上架但有型号的设备：按型号高度回填（型号删除则回退 height_u 原值即 1）
         UPDATE devices SET height_u = COALESCE(
             (SELECT dm.height_u FROM device_models dm WHERE dm.id = devices.device_model_id),
             devices.height_u
         )
          WHERE (start_u IS NULL OR end_u IS NULL OR end_u < start_u)
            AND device_model_id IS NOT NULL;

         PRAGMA user_version = 4;"
    )?;
    Ok(())
}

/// v4 → v5：为 4 张业务表补 `created_at` / `updated_at`，并给 `devices` 增加软删列 `deleted_at`。
///
/// 关键点（设计 §3.1）：
/// - SQLite 的 `ALTER TABLE ADD COLUMN` 不能带括号表达式默认值，故先加裸列再回填；
/// - 回填以"迁移时刻"为历史基准（UTC ISO8601），`updated_at` 初始等于 `created_at`；
/// - 【核心】设备两个 UNIQUE 索引重建为「部分索引」`WHERE col <> '' AND deleted_at IS NULL`，
///   使软删记录退出唯一性约束（软删后可用同序列号/资产号重建；恢复时由 db 层做冲突预检）；
/// - `uniq_rooms_name` / `uniq_device_models_name` 保持 v3 原样（二者仍为硬删除，无冲突）；
/// - 仅新增列 + 回填 + 用「更宽松」的部分索引替换旧索引，不会因既有数据失败。
fn migrate_v4_to_v5(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "-- 1) 时间戳列（ADD COLUMN 先加裸列，随后回填）
         ALTER TABLE devices       ADD COLUMN created_at TEXT;
         ALTER TABLE devices       ADD COLUMN updated_at TEXT;
         ALTER TABLE devices       ADD COLUMN deleted_at TEXT;   -- 软删除标记（仅 devices）
         ALTER TABLE rooms         ADD COLUMN created_at TEXT;
         ALTER TABLE rooms         ADD COLUMN updated_at TEXT;
         ALTER TABLE racks         ADD COLUMN created_at TEXT;
         ALTER TABLE racks         ADD COLUMN updated_at TEXT;
         ALTER TABLE device_models ADD COLUMN created_at TEXT;
         ALTER TABLE device_models ADD COLUMN updated_at TEXT;

         -- 2) 回填：老数据统一以迁移时刻为历史基准（UTC ISO8601）
         UPDATE devices       SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE created_at IS NULL;
         UPDATE rooms         SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE created_at IS NULL;
         UPDATE racks         SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE created_at IS NULL;
         UPDATE device_models SET created_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE created_at IS NULL;
         UPDATE devices       SET updated_at = created_at WHERE updated_at IS NULL;
         UPDATE rooms         SET updated_at = created_at WHERE updated_at IS NULL;
         UPDATE racks         SET updated_at = created_at WHERE updated_at IS NULL;
         UPDATE device_models SET updated_at = created_at WHERE updated_at IS NULL;

         -- 3) 【关键】设备唯一索引重建为部分索引（软删记录退出唯一性约束）
         DROP INDEX IF EXISTS uniq_devices_serial_no;
         CREATE UNIQUE INDEX uniq_devices_serial_no
             ON devices(serial_no) WHERE serial_no <> '' AND deleted_at IS NULL;
         DROP INDEX IF EXISTS uniq_devices_asset_no;
         CREATE UNIQUE INDEX uniq_devices_asset_no
             ON devices(asset_no) WHERE asset_no <> '' AND deleted_at IS NULL;

         -- 4) 新增索引（回收站过滤 / 排序 / 分页过滤）
         CREATE INDEX IF NOT EXISTS idx_devices_deleted_at   ON devices(deleted_at);
         CREATE INDEX IF NOT EXISTS idx_devices_name         ON devices(name);
         CREATE INDEX IF NOT EXISTS idx_devices_created_at   ON devices(created_at);
         CREATE INDEX IF NOT EXISTS idx_devices_updated_at   ON devices(updated_at);
         CREATE INDEX IF NOT EXISTS idx_devices_rack_deleted ON devices(rack_id, deleted_at);

         PRAGMA user_version = 5;"
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::{Connection, params};

    fn open_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        conn
    }

    fn index_names(conn: &Connection) -> Vec<String> {
        let mut stmt = conn
            .prepare("SELECT name FROM sqlite_master WHERE type IN ('index','table') AND name LIKE 'idx_%' OR (type='index' AND name LIKE 'uniq_%') ORDER BY name")
            .unwrap();
        let rows = stmt
            .query_map([], |r| r.get::<_, String>(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        rows
    }

    #[test]
    fn test_fresh_migration_reaches_v5_with_indexes_and_height_u() {
        let conn = open_db();
        run(&conn).unwrap();
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 5);
        let idx = index_names(&conn);
        // 7 个普通索引 + 4 个唯一索引（无重复数据时全部落地）
        assert!(idx.contains(&"idx_devices_rack_id".to_string()));
        assert!(idx.contains(&"idx_devices_device_model_id".to_string()));
        assert!(idx.contains(&"idx_devices_rack_name".to_string()));
        assert!(idx.contains(&"idx_racks_room_id".to_string()));
        assert!(idx.contains(&"idx_racks_sort_order".to_string()));
        assert!(idx.contains(&"idx_rooms_sort_order".to_string()));
        assert!(idx.contains(&"idx_device_models_name".to_string()));
        assert!(idx.contains(&"uniq_devices_serial_no".to_string()));
        assert!(idx.contains(&"uniq_devices_asset_no".to_string()));
        assert!(idx.contains(&"uniq_rooms_name".to_string()));
        assert!(idx.contains(&"uniq_device_models_name".to_string()));
        // v4: devices 表带 height_u 列
        let col: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('devices') WHERE name='height_u'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(col, 1);
    }

    #[test]
    fn test_v3_to_v4_backfills_device_height() {
        let conn = open_db();
        // 手工构造 v3 旧库（不执行 v4）
        migrate_v0_to_v1(&conn).unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        migrate_v2_to_v3(&conn).unwrap();

        // 型号（4U 服务器）与机柜（FK 约束需要）
        conn.execute(
            "INSERT INTO racks (name, height_u) VALUES ('R1', 42)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO device_models (name, type, height_u) VALUES ('M-4U', 'server', 4), ('M-1U', 'switch', 1)",
            [],
        )
        .unwrap();
        // 场景 1：在架多U设备（start=5,end=7 → 3U），无型号 → 按区间回填
        // 场景 2：未上架但有 4U 型号 → 按型号回填
        // 场景 3：未上架无型号 → 保持默认 1U
        conn.execute(
            "INSERT INTO devices (name, rack_id, start_u, end_u) VALUES ('A-Racked', 1, 5, 7)",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO devices (name, device_model_id) VALUES ('B-Pool-4U', 1)",
            [],
        )
        .unwrap();
        conn.execute("INSERT INTO devices (name) VALUES ('C-Pool-NoModel')", []).unwrap();

        migrate_v3_to_v4(&conn).unwrap();

        let rows: Vec<(String, i32)> = {
            let mut stmt = conn
                .prepare("SELECT name, height_u FROM devices ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        assert_eq!(rows[0], ("A-Racked".into(), 3));
        assert_eq!(rows[1], ("B-Pool-4U".into(), 4));
        assert_eq!(rows[2], ("C-Pool-NoModel".into(), 1));
        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 4);
    }

    #[test]
    fn test_unique_serial_blocks_duplicates() {
        let conn = open_db();
        run(&conn).unwrap();
        conn.execute(
            "INSERT INTO devices (name, serial_no) VALUES ('A', 'SN-1'), ('B', 'SN-2')",
            [],
        )
        .unwrap();
        // 重复非空序列号应被唯一索引拒绝
        let result = conn.execute("INSERT INTO devices (name, serial_no) VALUES ('C', 'SN-1')", []);
        assert!(result.is_err());
        // 空序列号不受约束
        conn.execute("INSERT INTO devices (name, serial_no) VALUES ('C', ''), ('D', '')", []).unwrap();
    }

    #[test]
    fn test_duplicate_serial_cleaned_before_unique_index() {
        let conn = open_db();
        // 手工构造 v2 旧库（不执行 v3）
        migrate_v0_to_v1(&conn).unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        // 旧库中存在重复非空序列号（v1/v2 时代无约束，可能产生）
        conn.execute(
            "INSERT INTO devices (name, serial_no) VALUES ('X1', 'SN-X'), ('X2', 'SN-X'), ('X3', 'SN-Y')",
            [],
        )
        .unwrap();

        // 执行 v2→v3：先清理重复，再建唯一索引
        migrate_v2_to_v3(&conn).unwrap();

        let rows: Vec<(String, String)> = {
            let mut stmt = conn
                .prepare("SELECT name, serial_no FROM devices ORDER BY id")
                .unwrap();
            stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
                .unwrap()
                .collect::<Result<_, _>>()
                .unwrap()
        };
        // X1 保留 SN-X（最早一条），X2 重复被清空，X3 保留 SN-Y
        assert_eq!(rows[0], ("X1".into(), "SN-X".into()));
        assert_eq!(rows[1], ("X2".into(), "".into()));
        assert_eq!(rows[2], ("X3".into(), "SN-Y".into()));
        // 唯一索引已建立：再插重复非空序列号应被拒绝
        let dup = conn.execute("INSERT INTO devices (name, serial_no) VALUES ('X4', 'SN-X')", []);
        assert!(dup.is_err());
    }

    #[test]
    fn test_migration_rollback_on_failure() {
        let conn = open_db();
        migrate_v0_to_v1(&conn).unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        let version_before: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version_before, 2);

        // 破坏性测试：让 v2→v3 中途失败（通过非法 SQL 模拟 —— 直接调 with_migration_tx 包一个必败函数）
        let result = with_migration_tx(&conn, |c| {
            c.execute_batch("CREATE TABLE _should_rollback (id INTEGER); INSERT INTO _should_rollback VALUES (1); SELECT * FROM nonexistent_table;")?;
            Ok(())
        });
        assert!(result.is_err());
        // 事务回滚后表不应存在
        let exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='_should_rollback'",
                [],
                |r| r.get(0),
            )
            .unwrap_or(false);
        assert!(!exists);
        // user_version 不被破坏（仍在 2）
        let version_after: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version_after, 2);
    }

    // ===================== v4 → v5（时间戳 + 软删 + 部分唯一索引）=====================

    fn table_has_column(conn: &Connection, table: &str, column: &str) -> bool {
        let count: i32 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM pragma_table_info('{}') WHERE name = ?1",
                    table
                ),
                params![column],
                |r| r.get(0),
            )
            .unwrap();
        count > 0
    }

    #[test]
    fn test_fresh_migration_reaches_v5() {
        let conn = open_db();
        run(&conn).unwrap();

        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 5);

        // devices 含三列时间戳 / 软删列
        assert!(table_has_column(&conn, "devices", "created_at"));
        assert!(table_has_column(&conn, "devices", "updated_at"));
        assert!(table_has_column(&conn, "devices", "deleted_at"));
        // rooms / racks / device_models 含两列时间戳
        for t in ["rooms", "racks", "device_models"] {
            assert!(table_has_column(&conn, t, "created_at"), "{} 缺 created_at", t);
            assert!(table_has_column(&conn, t, "updated_at"), "{} 缺 updated_at", t);
        }

        // 唯一索引仍存在，且设备两个索引已重建为「部分索引」（含 deleted_at IS NULL）
        let idx = index_names(&conn);
        assert!(idx.contains(&"uniq_devices_serial_no".to_string()));
        assert!(idx.contains(&"uniq_devices_asset_no".to_string()));
        let serial_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='index' AND name='uniq_devices_serial_no'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(serial_sql.contains("deleted_at IS NULL"));
        let asset_sql: String = conn
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='index' AND name='uniq_devices_asset_no'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(asset_sql.contains("deleted_at IS NULL"));

        // 5 个新增索引落地
        for name in [
            "idx_devices_deleted_at",
            "idx_devices_name",
            "idx_devices_created_at",
            "idx_devices_updated_at",
            "idx_devices_rack_deleted",
        ] {
            assert!(idx.contains(&name.to_string()), "缺索引 {}", name);
        }
    }

    #[test]
    fn test_v4_to_v5_backfills_timestamps() {
        let conn = open_db();
        // 手工构造 v4 旧库（不执行 v5）
        migrate_v0_to_v1(&conn).unwrap();
        migrate_v1_to_v2(&conn).unwrap();
        migrate_v2_to_v3(&conn).unwrap();
        migrate_v3_to_v4(&conn).unwrap();
        conn.execute("INSERT INTO devices (name) VALUES ('D1')", []).unwrap();
        conn.execute("INSERT INTO rooms (name) VALUES ('R1')", []).unwrap();
        conn.execute("INSERT INTO racks (name) VALUES ('K1')", []).unwrap();

        migrate_v4_to_v5(&conn).unwrap();

        let (d_created, d_updated): (String, String) = conn
            .query_row(
                "SELECT created_at, updated_at FROM devices WHERE name = 'D1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert!(!d_created.is_empty(), "created_at 应回填非空");
        assert_eq!(d_created, d_updated, "updated_at 初始应等于 created_at");

        for table in ["rooms", "racks"] {
            let (c, u): (String, String) = conn
                .query_row(
                    &format!("SELECT created_at, updated_at FROM {} LIMIT 1", table),
                    [],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .unwrap();
            assert!(!c.is_empty(), "{} created_at 应回填非空", table);
            assert_eq!(c, u, "{} updated_at 初始应等于 created_at", table);
        }

        let version: i32 = conn.pragma_query_value(None, "user_version", |r| r.get(0)).unwrap();
        assert_eq!(version, 5);
    }

    #[test]
    fn test_v5_soft_deleted_serial_can_be_recreated() {
        // 部分索引生效：软删 SN-1 后允许重建同序列号设备
        let conn = open_db();
        run(&conn).unwrap();
        conn.execute("INSERT INTO devices (name, serial_no) VALUES ('A', 'SN-1')", []).unwrap();
        conn.execute(
            "UPDATE devices SET deleted_at = strftime('%Y-%m-%dT%H:%M:%SZ','now') WHERE serial_no = 'SN-1'",
            [],
        )
        .unwrap();

        // 不应因唯一约束失败
        conn.execute("INSERT INTO devices (name, serial_no) VALUES ('A2', 'SN-1')", []).unwrap();
        let total: i32 = conn
            .query_row("SELECT COUNT(*) FROM devices WHERE serial_no = 'SN-1'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(total, 2, "软删记录与新建记录应共存");
        let active: i32 = conn
            .query_row(
                "SELECT COUNT(*) FROM devices WHERE serial_no = 'SN-1' AND deleted_at IS NULL",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(active, 1, "active 记录仍唯一");
    }

    #[test]
    fn test_v5_active_duplicate_serial_still_blocked() {
        // 部分索引仍约束 active 设备之间的唯一性
        let conn = open_db();
        run(&conn).unwrap();
        conn.execute("INSERT INTO devices (name, serial_no) VALUES ('A', 'SN-9')", []).unwrap();
        let dup = conn.execute("INSERT INTO devices (name, serial_no) VALUES ('B', 'SN-9')", []);
        assert!(dup.is_err(), "两条 active 同序列号应被拒");

        // 空串不受约束
        conn.execute("INSERT INTO devices (name, serial_no) VALUES ('C', ''), ('D', '')", []).unwrap();
    }
}
