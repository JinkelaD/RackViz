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
pub const LIVE_SCHEMA_VERSION: i32 = 5;

/// 待恢复暂存文件后缀：`<db>.restore-pending`（与主库同目录，保证同卷替换）。
const STAGING_SUFFIX: &str = ".restore-pending";
/// 待恢复操作标记文件名：`restore.pending`（与主库同目录）。
const MARKER_NAME: &str = "restore.pending";

/// 自动备份文件名前缀（与手动备份区分；滚动保留仅作用于该前缀）。
const AUTO_BACKUP_PREFIX: &str = "rackviz-auto-";

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

// ---------------------------------------------------------------------------
// A1：自动备份（每日去重 + 滚动保留）
// ---------------------------------------------------------------------------

/// 自动备份目录：`<主库同目录>/backups/`。
pub fn auto_backup_dir(db_path: &Path) -> PathBuf {
    match db_path.parent() {
        Some(dir) => dir.join("backups"),
        None => PathBuf::from("backups"),
    }
}

/// 目录下自动备份文件（`rackviz-auto-*.db`）按**文件名时间戳升序**排列。
fn sorted_auto_backups(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(AUTO_BACKUP_PREFIX) && n.ends_with(".db"))
                        .unwrap_or(false)
            })
            .collect(),
        Err(_) => Vec::new(),
    };
    // 文件名内嵌 UTC 时间戳（rackviz-auto-YYYYMMDD-HHMMSS.db），字典序即时间序
    files.sort();
    files
}

/// 最近一次自动备份文件的修改时间（无自动备份 → `None`）。
pub fn latest_auto_backup_time(dir: &Path) -> Option<std::time::SystemTime> {
    sorted_auto_backups(dir)
        .last()
        .and_then(|p| fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
}

/// 为自动备份生成本次目标路径：`<dir>/rackviz-auto-YYYYMMDD-HHMMSS.db`。
pub fn next_auto_backup_path(dir: &Path) -> PathBuf {
    dir.join(format!(
        "{}{}.db",
        AUTO_BACKUP_PREFIX,
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    ))
}

/// 滚动保留：按文件名时间戳升序仅保留最近 `keep` 份自动备份，
/// 返回被删除的文件名列表（删除失败仅告警，不阻断）。
pub fn prune_auto_backups(dir: &Path, keep: usize) -> Vec<String> {
    let files = sorted_auto_backups(dir);
    if files.len() <= keep {
        return Vec::new();
    }
    let to_delete = &files[..files.len() - keep];
    to_delete
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            match fs::remove_file(p) {
                Ok(()) => {
                    log::info!("[A1] 滚动保留清理旧自动备份: {}", name);
                    Some(name)
                }
                Err(e) => {
                    log::warn!("[A1] 清理旧自动备份失败（忽略）: {} - {}", name, e);
                    None
                }
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// A4：启动完整性自检
// ---------------------------------------------------------------------------

/// 启动完整性自检：`PRAGMA integrity_check` == ok 且 `user_version <= max_version`。
///
/// 失败返回可读错误（供前端引导用户到备份管理恢复）。
pub fn check_db_integrity(conn: &Connection, max_version: i32) -> Result<(), AppError> {
    let integrity: String = conn
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| AppError::io(&format!("integrity_check 执行失败: {}", e)))?;
    if integrity != "ok" {
        return Err(AppError::validation(&format!(
            "数据库结构损坏（integrity_check: {}）",
            integrity
        )));
    }
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap_or(0);
    if version > max_version {
        return Err(AppError::validation(&format!(
            "数据库版本（v{}）高于当前程序支持版本（v{}）",
            version, max_version
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// A2：备份列表 / 删除
// ---------------------------------------------------------------------------

/// 列出备份目录中的自动备份文件信息（含有效性校验；按修改时间倒序）。
///
/// 单个文件校验失败不阻断整体（valid=false 照常列出，由用户决定删除）。
pub fn list_backup_infos(dir: &Path) -> Vec<crate::models::BackupInfo> {
    let mut infos: Vec<crate::models::BackupInfo> = sorted_auto_backups(dir)
        .iter()
        .filter_map(|p| {
            let name = p.file_name()?.to_str()?.to_string();
            let meta = fs::metadata(p).ok()?;
            let modified_at = meta
                .modified()
                .ok()
                .map(|t| chrono::DateTime::<chrono::Utc>::from(t).to_rfc3339())
                .unwrap_or_default();
            let valid = validate_backup(p).is_ok();
            Some(crate::models::BackupInfo {
                full_path: p.to_string_lossy().to_string(),
                name,
                size_bytes: meta.len() as i64,
                modified_at,
                valid,
            })
        })
        .collect();
    infos.reverse(); // 倒序：最新在前
    infos
}

/// 删除备份文件：名称白名单校验（仅允许 `rackviz-auto-*.db`），杜绝路径穿越。
pub fn delete_backup_file(dir: &Path, name: &str) -> Result<(), AppError> {
    let legal =
        name.starts_with(AUTO_BACKUP_PREFIX) && name.ends_with(".db") && !name.contains(['/', '\\']);
    if !legal {
        return Err(AppError::validation("非法的备份文件名"));
    }
    let path = dir.join(name);
    if !path.is_file() {
        return Err(AppError::validation("备份文件不存在"));
    }
    fs::remove_file(&path)?;
    log::info!("[操作] 已删除备份: {}", name);
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

    /// 点 8①：损坏文件（非 SQLite / 截断）必须被 `validate_backup` 拒绝。
    #[test]
    fn test_validate_backup_rejects_corrupted_file() {
        let dir = temp_dir("corrupt");
        // (a) 任意文本文件（非 SQLite）
        let txt = dir.join("garbage.db");
        fs::write(&txt, b"this is definitely not a sqlite database file").unwrap();
        assert!(validate_backup(&txt).is_err(), "非 SQLite 文本文件应被拒绝");

        // (b) 截断的合法库（仅保留头部若干字节 → 结构损坏）
        let good = dir.join("good.db");
        {
            let conn = open_seeded(&good);
            let _ = conn;
        }
        let bytes = fs::read(&good).unwrap();
        let truncated = dir.join("truncated.db");
        fs::write(&truncated, &bytes[..bytes.len().min(200)]).unwrap();
        assert!(validate_backup(&truncated).is_err(), "截断的库文件应被拒绝");

        let _ = fs::remove_dir_all(&dir);
    }

    /// 点 8①：`user_version` 高于当前程序支持版本的备份必须拒绝（防降级破坏数据）。
    #[test]
    fn test_validate_backup_rejects_higher_user_version() {
        let dir = temp_dir("highver");
        let db = dir.join("future.db");
        {
            let conn = Connection::open(&db).unwrap();
            conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
            migration::run(&conn).unwrap();
            conn.execute("INSERT INTO devices (name) VALUES ('X')", []).unwrap();
            // 伪造一个「未来版本」备份
            conn.pragma_update(None, "user_version", 6).unwrap();
        }
        assert!(
            validate_backup(&db).is_err(),
            "版本高于当前程序（v6 > v5）的备份必须被拒绝"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    /// 点 8③：换库时必须清理主库残留的 `-wal` / `-shm`，否则旧 WAL 会覆盖恢复后的数据。
    #[test]
    fn test_apply_pending_restore_cleans_wal_and_shm() {
        let dir = temp_dir("walshm");
        let db_path = dir.join("rackviz.db");
        {
            let conn = open_seeded(&db_path);
            let _ = conn;
        }
        // 制造残留的 -wal / -shm
        let wal = with_suffix(&db_path, "-wal");
        let shm = with_suffix(&db_path, "-shm");
        fs::write(&wal, b"stale-wal").unwrap();
        fs::write(&shm, b"stale-shm").unwrap();
        assert!(wal.exists() && shm.exists());

        // 构造含 2 台设备的备份并暂存
        let src_db = dir.join("src.db");
        {
            let conn = Connection::open(&src_db).unwrap();
            migration::run(&conn).unwrap();
            conn.execute("INSERT INTO devices (name) VALUES ('A'), ('B')", []).unwrap();
            create_backup(&conn, &dir.join("src_vacuum.db")).unwrap();
        }
        stage_restore(&db_path, &dir.join("src_vacuum.db")).unwrap();
        apply_pending_restore(&db_path).unwrap();

        assert!(!wal.exists(), "换库后必须删除 -wal");
        assert!(!shm.exists(), "换库后必须删除 -shm");
        let conn = Connection::open(&db_path).unwrap();
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM devices", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 2, "主库应被替换为备份内容");
        let _ = fs::remove_dir_all(&dir);
    }

    /// A1：滚动保留——10 份自动备份 prune(7) 后应剩最新的 7 份。
    #[test]
    fn test_prune_auto_backups_keeps_latest_n() {
        let dir = temp_dir("prune");
        // 文件名内嵌 UTC 时间戳，字典序即时间序；造 10 份（2026-09-17 00:00 ~ 00:09）
        for i in 0..10 {
            let name = format!("rackviz-auto-20260917-00000{}.db", i);
            fs::write(dir.join(&name), b"stub").unwrap();
        }
        // 非自动备份文件不应参与清理
        fs::write(dir.join("rackviz-backup-manual.db"), b"stub").unwrap();

        let deleted = prune_auto_backups(&dir, 7);
        assert_eq!(deleted.len(), 3, "应删除最旧的 3 份");
        assert!(deleted.contains(&"rackviz-auto-20260917-000000.db".to_string()));
        assert!(deleted.contains(&"rackviz-auto-20260917-000002.db".to_string()));

        let remain = sorted_auto_backups(&dir);
        assert_eq!(remain.len(), 7, "应保留 7 份");
        // 最新 3 份（000007-000009）必须在
        for i in 7..10 {
            let name = format!("rackviz-auto-20260917-00000{}.db", i);
            assert!(dir.join(&name).exists(), "{} 应保留", name);
        }
        // 手动备份不受影响
        assert!(dir.join("rackviz-backup-manual.db").exists());
        // 再 prune：不足 keep 数时应为 no-op
        assert!(prune_auto_backups(&dir, 7).is_empty());

        let _ = fs::remove_dir_all(&dir);
    }

    /// A1：去重窗口判断——latest_auto_backup_time 返回最新文件 mtime。
    #[test]
    fn test_latest_auto_backup_time() {
        let dir = temp_dir("latest");
        assert!(latest_auto_backup_time(&dir).is_none(), "空目录应返回 None");
        fs::write(dir.join("rackviz-auto-20260917-000000.db"), b"stub").unwrap();
        assert!(latest_auto_backup_time(&dir).is_some());
        let _ = fs::remove_dir_all(&dir);
    }

    /// A4：完整性自检——健康库 Ok；user_version 超上限的库 Err。
    #[test]
    fn test_check_db_integrity() {
        let dir = temp_dir("integrity");
        let db_path = dir.join("rackviz.db");
        {
            let conn = open_seeded(&db_path);
            check_db_integrity(&conn, LIVE_SCHEMA_VERSION).expect("健康库应通过自检");
        }
        // 伪造未来版本
        let db2 = dir.join("future.db");
        {
            let conn = Connection::open(&db2).unwrap();
            migration::run(&conn).unwrap();
            conn.pragma_update(None, "user_version", 6).unwrap();
            assert!(check_db_integrity(&conn, LIVE_SCHEMA_VERSION).is_err());
        }
        let _ = fs::remove_dir_all(&dir);
    }

    /// A2：删除备份的白名单校验——拒绝路径穿越与非自动备份名。
    #[test]
    fn test_delete_backup_name_whitelist() {
        let dir = temp_dir("delbackup");
        fs::write(dir.join("rackviz-auto-20260917-000000.db"), b"stub").unwrap();

        // 非法名：穿越 / 非前缀 / 非后缀
        assert!(delete_backup_file(&dir, "../rackviz.db").is_err());
        assert!(delete_backup_file(&dir, "rackviz.db").is_err());
        assert!(delete_backup_file(&dir, "rackviz-auto-x.txt").is_err());
        // 合法名
        delete_backup_file(&dir, "rackviz-auto-20260917-000000.db").unwrap();
        assert!(!dir.join("rackviz-auto-20260917-000000.db").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
