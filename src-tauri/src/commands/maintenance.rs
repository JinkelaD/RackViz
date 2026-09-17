//! 维护类命令（N-18 数据备份 / 恢复）。
//!
//! 非阻塞红线（审查 P-4/B-04）：保存对话框沿用 `exports.rs` 的
//! 「非阻塞 dialog + 独立线程等待」模式；`VACUUM INTO` / 文件复制等重 I/O
//! 一律放入 `spawn_blocking`，避免冻结 UI 线程。

use std::path::{Path, PathBuf};

use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::backup;
use crate::error::AppError;
use crate::models::{BackupInfo, DbHealth, RestoreResult};
use crate::state::DbState;

/// 恢复前自动备份（安全网）的存放子目录名（位于 app_local_data 下）。
const AUTO_BACKUP_DIR: &str = "backups";
/// 主库文件名（与 `lib.rs` setup 一致）。
const DB_FILE_NAME: &str = "rackviz.db";
/// A1：自动备份去重窗口（小时内已有自动备份则跳过）。
const AUTO_BACKUP_INTERVAL: std::time::Duration = std::time::Duration::from_secs(24 * 3600);
/// A1：自动备份滚动保留份数。
const AUTO_BACKUP_KEEP: usize = 7;

fn save_path_to_string(path: Option<tauri_plugin_dialog::FilePath>) -> Result<PathBuf, AppError> {
    let fp = path.ok_or_else(|| AppError::cancelled("用户取消了保存"))?;
    match fp {
        tauri_plugin_dialog::FilePath::Path(p) => Ok(p),
        tauri_plugin_dialog::FilePath::Url(_) => Err(AppError::io("不支持URL路径")),
    }
}

/// 在后台线程弹出非阻塞原生保存对话框并等待结果（不冻结 UI 线程）。
async fn save_file_dialog_async(
    app: &tauri::AppHandle,
    filter_name: &str,
    extensions: &[&str],
    file_name: &str,
) -> Result<PathBuf, AppError> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<tauri_plugin_dialog::FilePath>>();
    let app = app.clone();
    app.dialog()
        .file()
        .add_filter(filter_name, extensions)
        .set_file_name(file_name)
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    let result = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| AppError::io(&format!("保存对话框异常: {}", e)))?;
    match result {
        Ok(Some(path)) => save_path_to_string(Some(path)),
        Ok(None) => Err(AppError::cancelled("用户取消了保存")),
        Err(_) => Err(AppError::cancelled("用户取消了保存")),
    }
}

/// 推导主库路径 `<app_local_data>/rackviz.db`。
fn resolve_db_path(app: &tauri::AppHandle) -> Result<PathBuf, AppError> {
    app.path()
        .app_local_data_dir()
        .map(|dir| dir.join(DB_FILE_NAME))
        .map_err(|e| AppError::io(&format!("获取应用数据目录失败: {}", e)))
}

/// 备份数据库：弹原生保存对话框选路径 → `VACUUM INTO` 生成一致性单文件 → 返回路径。
#[tauri::command]
pub async fn backup_database(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let default_name = format!(
        "rackviz-backup-{}.db",
        chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
    let path = save_file_dialog_async(&app, "SQLite 数据库", &["db"], &default_name).await?;

    let pool = state.pool.clone();
    let task_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        let conn = pool.get()?;
        backup::create_backup(&conn, &task_path)?;
        log::info!("[操作] 数据库备份完成: {:?}", task_path);
        Ok(())
    })
    .await
    .map_err(|e| AppError::io(&format!("备份任务异常: {}", e)))??;

    Ok(path.to_string_lossy().to_string())
}

/// 从备份恢复：校验所选备份 → 自动备份当前库 → 暂存（`restore.pending`），
/// 返回 `restart_required=true`，由下次启动在开池前完成替换。
#[tauri::command]
pub async fn restore_database(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    path: String,
) -> Result<RestoreResult, AppError> {
    let source = PathBuf::from(&path);
    let db_path = resolve_db_path(&app)?;
    let pool = state.pool.clone();

    tauri::async_runtime::spawn_blocking(move || -> Result<(), AppError> {
        // 1) 校验备份（损坏 / 非本应用 / 版本过高 → 直接拒绝）
        backup::validate_backup(&source)?;

        // 2) 恢复前自动备份当前库（尽力而为：失败仅告警，不阻断恢复）
        let auto_dir = db_path
            .parent()
            .map(|d| d.join(AUTO_BACKUP_DIR))
            .unwrap_or_else(|| PathBuf::from(AUTO_BACKUP_DIR));
        let _ = std::fs::create_dir_all(&auto_dir);
        let auto_path = auto_dir.join(format!(
            "rackviz-auto-{}.db",
            chrono::Utc::now().format("%Y%m%d-%H%M%S")
        ));
        match pool.get() {
            Ok(conn) => match backup::create_backup(&conn, &auto_path) {
                Ok(()) => log::info!("[操作] 恢复前自动备份: {:?}", auto_path),
                Err(e) => log::warn!("恢复前自动备份失败（继续恢复）: {}", e),
            },
            Err(e) => log::warn!("恢复前自动备份跳过（取连接失败）: {}", e),
        }

        // 3) 暂存：写暂存文件 + restore.pending 标记，重启后由 DbState::new 应用
        backup::stage_restore(&db_path, &source)?;
        Ok(())
    })
    .await
    .map_err(|e| AppError::io(&format!("恢复任务异常: {}", e)))??;

    log::info!("[操作] 已排期数据库恢复，重启后生效: {}", path);
    Ok(RestoreResult {
        restart_required: true,
        message: "恢复已准备完成：当前数据库已自动备份，请重启应用以生效。".to_string(),
    })
}

/// A1：自动备份（每日去重 + 滚动保留 7 份）。
///
/// 前端启动时与运行期定时调用；24 小时内已有自动备份则跳过（`performed=false`）。
/// 失败不阻断应用使用（返回 Err 由前端提示一次即可）。
#[tauri::command]
pub async fn auto_backup(state: State<'_, DbState>) -> Result<RestoreResult, AppError> {
    let pool = state.pool.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<(bool, String), AppError> {
        let conn = pool.get()?;
        let db_path = conn
            .path()
            .map(PathBuf::from)
            .ok_or_else(|| AppError::io("无法获取主库路径"))?;
        let dir = backup::auto_backup_dir(&db_path);
        std::fs::create_dir_all(&dir)?;

        // 去重：24h 内已有自动备份则跳过
        if let Some(t) = backup::latest_auto_backup_time(&dir) {
            if let Ok(age) = std::time::SystemTime::now().duration_since(t) {
                if age < AUTO_BACKUP_INTERVAL {
                    return Ok((false, "今日已自动备份，跳过".to_string()));
                }
            }
        }

        let dest = backup::next_auto_backup_path(&dir);
        backup::create_backup(&conn, &dest)?;
        let pruned = backup::prune_auto_backups(&dir, AUTO_BACKUP_KEEP);
        log::info!(
            "[A1] 自动备份完成: {:?}（清理 {} 份旧备份）",
            dest,
            pruned.len()
        );
        Ok((true, format!("自动备份完成：{}", dest.file_name().unwrap_or_default().to_string_lossy())))
    })
    .await
    .map_err(|e| AppError::io(&format!("自动备份任务异常: {}", e)))??;

    Ok(RestoreResult {
        restart_required: false,
        message: result.1,
    })
}

/// A2：列出自动备份文件信息（最新在前，含有效性校验）。
#[tauri::command]
pub async fn list_backups(app: tauri::AppHandle) -> Result<Vec<BackupInfo>, AppError> {
    let db_path = resolve_db_path(&app)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let dir = backup::auto_backup_dir(&db_path);
        backup::list_backup_infos(&dir)
    })
    .await
    .map_err(|e| AppError::io(&format!("备份列表任务异常: {}", e)))?;
    Ok(result)
}

/// A2：删除指定自动备份文件（名称白名单校验）。
#[tauri::command]
pub async fn delete_backup(app: tauri::AppHandle, name: String) -> Result<(), AppError> {
    let db_path = resolve_db_path(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        let dir = backup::auto_backup_dir(&db_path);
        backup::delete_backup_file(&dir, &name)
    })
    .await
    .map_err(|e| AppError::io(&format!("删除备份任务异常: {}", e)))?
}

/// A4：启动完整性自检（integrity_check + schema 版本上限）。
///
/// 前端启动时查询；异常时引导用户到备份管理恢复。
#[tauri::command]
pub async fn get_db_health(state: State<'_, DbState>) -> Result<DbHealth, AppError> {
    let pool = state.pool.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> DbHealth {
        let health_from = |r: Result<(), AppError>, count: i64| DbHealth {
            ok: r.is_ok(),
            message: match &r {
                Ok(()) => String::new(),
                Err(e) => e.to_string(),
            },
            backup_count: count,
        };
        match pool.get() {
            Ok(conn) => {
                let r = crate::backup::check_db_integrity(&conn, crate::backup::LIVE_SCHEMA_VERSION);
                // 备份数尽力统计（失败计 0）
                let count = conn
                    .path()
                    .map(|p| {
                        let dir = backup::auto_backup_dir(Path::new(p));
                        backup::list_backup_infos(&dir).len() as i64
                    })
                    .unwrap_or(0);
                health_from(r, count)
            }
            Err(e) => DbHealth {
                ok: false,
                message: format!("获取数据库连接失败: {}", e),
                backup_count: 0,
            },
        }
    })
    .await
    .map_err(|e| AppError::io(&format!("完整性自检任务异常: {}", e)))?;
    Ok(result)
}
