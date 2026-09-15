//! 维护类命令（N-18 数据备份 / 恢复）。
//!
//! 非阻塞红线（审查 P-4/B-04）：保存对话框沿用 `exports.rs` 的
//! 「非阻塞 dialog + 独立线程等待」模式；`VACUUM INTO` / 文件复制等重 I/O
//! 一律放入 `spawn_blocking`，避免冻结 UI 线程。

use std::path::PathBuf;

use tauri::{Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::backup;
use crate::error::AppError;
use crate::models::RestoreResult;
use crate::state::DbState;

/// 恢复前自动备份（安全网）的存放子目录名（位于 app_local_data 下）。
const AUTO_BACKUP_DIR: &str = "backups";
/// 主库文件名（与 `lib.rs` setup 一致）。
const DB_FILE_NAME: &str = "rackviz.db";

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
