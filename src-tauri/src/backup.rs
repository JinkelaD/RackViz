//! 数据备份与恢复（N-18）。
//!
//! 设计要点（对齐 v2.0.0 架构设计 §9 / §10）：
//! - **备份**：`VACUUM INTO` 生成**自包含一致性单文件**，规避 WAL 模式下直接拷贝
//!   `rackviz.db` 可能丢失未 checkpoint 事务的问题。
//! - **恢复（延迟交换）**：运行期有多个连接（r2d2 池大小 4），禁止原地覆盖主库。
//!   故选定的备份先暂存为 `<db>.restore-pending` 并写 `restore.pending` 标记，
//!   由 `DbState::new` 在**打开连接池之前**调用 [`apply_pending_restore`] 完成替换。
//!
//! 所有函数均为纯同步（重 I/O），由命令层在 `spawn_blocking` 中调用。

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection, OpenFlags};

use crate::error::AppError;

/// 主库 schema 版本上限（与 `migration::CURRENT_VERSION` 对齐）。
/// 更高版本的备份文件拒绝恢复，避免「降级」破坏数据。
const LIVE_SCHEMA_VERSION: i32 = 5;

/// 待恢复暂存文件后缀：`<db>.restore-pending`（与主库同目录，保证同卷替换）。
const STAGING_SUFFIX: &str = ".restore-pending";
/// 待恢复操作标记文件名：`restore.pending`（与主库同目录）。
const MARKER_NAME: &str = "restore.pending";

/// 在路径尾部追加后缀（基于 `OsStr`，避免非 UTF-8 路径信息丢失）。
fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut s = path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// 暂存文件路径 `<db>.restore-pending`。
fn staging_path(db_path: &Path) -> PathBuf {
    with_suffix(db_path, STAGING_SUFFIX)
}

/// 标记文件路径（与主库同目录）：`<dir>/restore.pending`。
fn marker_path(db_path: &Path) -> PathBuf {
    match db_path.parent() {
        Some(dir) => dir.join(MARKER_NAME),
        None => PathBuf::from(MARKER_NAME),
    }
}

/// 使用 `VACUUM INTO` 生成当前库的一致性快照到 `dest`。
///
/// `VACUUM INTO` 要求目标文件**不存在**，故先尽力删除同名文件。
/// 必须在**事务之外**执行（本函数不开启事务）。
pub fn create_backup(conn: &Connection, dest: &Path) -> Result<(), AppError> {
    if dest.exists() {
        fs::remove_file(dest)?;
    }
    let dest_str = dest
        .to_str()
        .ok_or_else(|| AppError::io("备份路径包含非法字符"))?;
    conn.execute("VACUUM INTO ?1", params![dest_str])?;
    Ok(())
}

/// 校验备份文件是否为**可恢复的有效快照**：
/// ① 文件存在；② `PRAGMA integrity_check` == `ok`（结构完整）；
/// ③ 含 `devices` 表（确属 RackViz 备份）；④ `user_version <= LIVE_SCHEMA_VERSION`。
pub fn validate_backup(path: &Path) -> Result<(), AppError> {
    if !path.exists() {
        return Err(AppError::validation("备份文件不存在"));
    }
    // 只读打开，避免误改备份文件。
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(AppError::validation(&format!(
            "备份文件已损坏（integrity_check: {}）",
            integrity
        )));
    }

    let has_devices: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'devices'",
        [],
        |r| r.get(0),
    )?;
    if has_devices == 0 {
        return Err(AppError::validation(
            "备份文件缺少 devices 表，不是有效的 RackViz 备份",
        ));
    }

    let version: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap_or(0);
    if version > LIVE_SCHEMA_VERSION {
        return Err(AppError::validation(&format!(
            "备份文件版本（v{}）高于当前程序支持版本（v{}），请升级程序后再恢复",
            version, LIVE_SCHEMA_VERSION
        )));
    }
    Ok(())
}

/// 暂存恢复：把所选备份复制到 `<db>.restore-pending` 并写 `restore.pending` 标记。
///
/// 实际替换延迟到下次启动（见 [`apply_pending_restore`]），运行期不触碰主库。
pub fn stage_restore(db_path: &Path, source: &Path) -> Result<(), AppError> {
    if !source.exists() {
        return Err(AppError::validation("备份文件不存在"));
    }
    let staging = staging_path(db_path);
    fs::copy(source, &staging)?;
    fs::write(marker_path(db_path), b"pending\n")?;
    Ok(())
}

/// 启动时应用待恢复（**必须在打开连接池之前**调用）：
/// 清理主库的 `-wal` / `-shm`，用暂存文件替换主库，成功后删除暂存与标记。
///
/// 采用「主库先改名→`.old`，再把暂存复制就位」的策略；
/// 复制失败时用 `.old` 回滚，杜绝主库丢失。
pub fn apply_pending_restore(db_path: &Path) -> std::io::Result<()> {
    let marker = marker_path(db_path);
    if !marker.exists() {
        return Ok(());
    }
    let staging = staging_path(db_path);
    if !staging.exists() {
        // 标记残留但暂存缺失（上次复制前中断）：清理陈旧标记，正常启动。
        log::warn!("发现 {} 标记但缺少暂存文件，清理陈旧标记", MARKER_NAME);
        let _ = fs::remove_file(&marker);
        return Ok(());
    }

    // 1) 清理 WAL / SHM，否则旧 WAL 会覆盖恢复后的主库。
    for suffix in ["-wal", "-shm"] {
        let p = with_suffix(db_path, suffix);
        if p.exists() {
            let _ = fs::remove_file(&p);
        }
    }

    // 2) 主库先改名暂存（`.old`），再复制暂存文件就位。
    let old = with_suffix(db_path, ".old");
    if old.exists() {
        let _ = fs::remove_file(&old);
    }
    let had_main = db_path.exists();
    if had_main {
        fs::rename(db_path, &old)?;
    }

    match fs::copy(&staging, db_path) {
        Ok(_) => {
            if old.exists() {
                let _ = fs::remove_file(&old);
            }
            let _ = fs::remove_file(&staging);
            let _ = fs::remove_file(&marker);
            log::info!("待恢复数据库已应用: {:?}", db_path);
            Ok(())
        }
        Err(e) => {
            // 回滚：恢复原主库，保留标记以便下次重试。
            if old.exists() {
                let _ = fs::copy(&old, db_path);
                let _ = fs::remove_file(&old);
            }
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;

    fn temp_dir(tag: &str) -> PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!("rackviz_backup_test_{}_{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn open_seeded(db_path: &Path) -> Connection {
        let conn = Connection::open(db_path).unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        migration::run(&conn).unwrap();
        conn.execute("INSERT INTO devices (name) VALUES ('Seed')", []).unwrap();
        conn
    }

    #[test]
    fn test_create_backup_produces_valid_snapshot() {
        let dir = temp_dir("create");
        let db_path = dir.join("rackviz.db");
        let conn = open_seeded(&db_path);

        let dest = dir.join("out.db");
        create_backup(&conn, &dest).unwrap();
        assert!(dest.exists(), "备份文件应生成");

        // 生成的快照应是有效备份
        validate_backup(&dest).unwrap();

        // 快照内含 seed 数据
        let snap = Connection::open_with_flags(&dest, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let count: i64 = snap
            .query_row("SELECT COUNT(*) FROM devices", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_backup_rejects_missing_and_non_backup() {
        let dir = temp_dir("invalid");
        // 不存在
        assert!(validate_backup(&dir.join("nope.db")).is_err());

        // 存在但不是有效 SQLite（缺 devices 表）→ 校验失败
        let bad = dir.join("bad.db");
        {
            let conn = Connection::open(&bad).unwrap();
            conn.execute_batch("CREATE TABLE other (id INTEGER);").unwrap();
        }
        assert!(validate_backup(&bad).is_err());

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_stage_and_apply_pending_restore() {
        let dir = temp_dir("restore");
        let db_path = dir.join("rackviz.db");

        // 当前主库：1 台设备
        {
            let conn = open_seeded(&db_path);
            let _ = conn;
        }

        // 构造一个含 2 台设备的备份，并暂存
        let backup_db = dir.join("src.db");
        {
            let conn = Connection::open(&backup_db).unwrap();
            migration::run(&conn).unwrap();
            conn.execute(
                "INSERT INTO devices (name) VALUES ('A'), ('B')",
                [],
            )
            .unwrap();
            create_backup(&conn, &dir.join("src_vacuum.db")).unwrap();
        }
        let staged_src = dir.join("src_vacuum.db");
        stage_restore(&db_path, &staged_src).unwrap();
        assert!(marker_path(&db_path).exists(), "应写入 restore.pending 标记");
        assert!(staging_path(&db_path).exists(), "应写入暂存文件");

        // 应用待恢复
        apply_pending_restore(&db_path).unwrap();
        assert!(!marker_path(&db_path).exists(), "应用后应删除标记");
        assert!(!staging_path(&db_path).exists(), "应用后应删除暂存");

        // 主库已被替换为备份内容（2 台设备）
        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM devices", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2, "主库应被替换为备份内容");

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_apply_without_marker_is_noop() {
        let dir = temp_dir("noop");
        let db_path = dir.join("rackviz.db");
        // 无标记：应为 no-op 且不报错
        apply_pending_restore(&db_path).unwrap();
        assert!(!db_path.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
