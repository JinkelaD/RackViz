use tauri::State;
use crate::state::DbState;
use crate::error::AppError;
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoggingConfig {
    pub enabled: bool,
    pub log_dir: String,
}

#[tauri::command]
pub fn get_logging_config(state: State<'_, DbState>) -> Result<LoggingConfig, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let enabled = crate::db::settings::is_logging_enabled(&conn)?;
    let log_dir = state.log_dir().to_string_lossy().to_string();
    Ok(LoggingConfig { enabled, log_dir })
}

#[tauri::command]
pub fn set_logging_enabled(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    enabled: bool,
) -> Result<LoggingConfig, AppError> {
    {
        let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
        crate::db::settings::set_logging_enabled(&conn, enabled)?;
    }
    // 动态切换日志输出
    crate::logging::reconfigure(app, enabled)?;
    log::info!("[操作] 日志收集已{}", if enabled { "开启" } else { "关闭" });
    let log_dir = state.log_dir().to_string_lossy().to_string();
    Ok(LoggingConfig { enabled, log_dir })
}

#[tauri::command]
pub fn open_log_dir(state: State<'_, DbState>) -> Result<(), AppError> {
    let dir = state.log_dir();
    std::fs::create_dir_all(&dir)?;
    // 用系统默认程序打开目录
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg(&dir)
            .spawn()
            .map_err(|e| AppError::io(&format!("无法打开目录: {}", e)))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir)
            .spawn()
            .map_err(|e| AppError::io(&format!("无法打开目录: {}", e)))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir)
            .spawn()
            .map_err(|e| AppError::io(&format!("无法打开目录: {}", e)))?;
    }
    Ok(())
}
