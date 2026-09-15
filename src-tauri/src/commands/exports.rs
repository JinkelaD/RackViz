use tauri::State;
use crate::state::DbState;
use crate::error::AppError;
use crate::models::{ImportOptions, ImportResult};
use tauri_plugin_dialog::DialogExt;
use std::path::PathBuf;

fn save_path_to_string(path: Option<tauri_plugin_dialog::FilePath>) -> Result<PathBuf, AppError> {
    let fp = path.ok_or_else(|| AppError::cancelled("用户取消了保存"))?;
    match fp {
        tauri_plugin_dialog::FilePath::Path(p) => Ok(p),
        tauri_plugin_dialog::FilePath::Url(_) => Err(AppError::io("不支持URL路径")),
    }
}

/// 在后台线程弹出非阻塞原生保存对话框并等待结果。
/// 非阻塞对话框 + 独立线程等待 = UI 线程与事件循环不被冻结（审查红线 P-4/B-04）。
async fn save_file_dialog_async(
    app: &tauri::AppHandle,
    filter_name: &str,
    extensions: &[&str],
    file_name: &str,
) -> Result<PathBuf, AppError> {
    let (tx, rx) = std::sync::mpsc::channel::<Option<tauri_plugin_dialog::FilePath>>();
    let app = app.clone();
    // 打开原生对话框（插件内部在独立线程运行，非阻塞返回）
    app.dialog()
        .file()
        .add_filter(filter_name, extensions)
        .set_file_name(file_name)
        .save_file(move |path| {
            let _ = tx.send(path);
        });

    // 独立线程等待用户选择，避免占用 async runtime
    let result = tauri::async_runtime::spawn_blocking(move || rx.recv())
        .await
        .map_err(|e| AppError::io(&format!("保存对话框异常: {}", e)))?;
    match result {
        Ok(Some(path)) => save_path_to_string(Some(path)),
        Ok(None) => Err(AppError::cancelled("用户取消了保存")),
        Err(_) => Err(AppError::cancelled("用户取消了保存")),
    }
}

#[tauri::command]
pub async fn export_racks_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    // 先生成文件内容（DB 读取 + Excel 生成放后台线程）
    let pool = state.pool.clone();
    let data = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
        let conn = pool.get()?;
        crate::excel::export_racks_excel(&conn)
    })
    .await
    .map_err(|e| AppError::io(&format!("导出数据生成异常: {}", e)))??;

    let path = save_file_dialog_async(&app, "Excel", &["xlsx"], "机柜部署图.xlsx").await?;
    let task_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&task_path, data)?;
        log::info!("[操作] 导出机柜部署图: {:?}", task_path);
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|e| AppError::io(&format!("导出任务异常: {}", e)))??;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_devices_data_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let pool = state.pool.clone();
    let data = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
        let conn = pool.get()?;
        crate::excel::export_devices_data_excel(&conn)
    })
    .await
    .map_err(|e| AppError::io(&format!("导出数据生成异常: {}", e)))??;

    let path = save_file_dialog_async(&app, "Excel", &["xlsx"], "设备台账.xlsx").await?;
    let task_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&task_path, data)?;
        log::info!("[操作] 导出设备台账: {:?}", task_path);
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|e| AppError::io(&format!("导出任务异常: {}", e)))??;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_single_rack_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    rack_id: i32,
) -> Result<String, AppError> {
    let pool = state.pool.clone();
    let data = tauri::async_runtime::spawn_blocking(move || -> Result<Vec<u8>, AppError> {
        let conn = pool.get()?;
        crate::excel::export_single_rack_excel(&conn, rack_id)
    })
    .await
    .map_err(|e| AppError::io(&format!("导出数据生成异常: {}", e)))??;

    let path = save_file_dialog_async(&app, "Excel", &["xlsx"], &format!("机柜_{}.xlsx", rack_id)).await?;
    let task_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&task_path, data)?;
        log::info!("[操作] 导出单机柜Excel: rack_id={}, path={:?}", rack_id, task_path);
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|e| AppError::io(&format!("导出任务异常: {}", e)))??;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn export_report_html(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let pool = state.pool.clone();
    let html = tauri::async_runtime::spawn_blocking(move || -> Result<String, AppError> {
        let conn = pool.get()?;
        crate::report::render_report(&conn)
    })
    .await
    .map_err(|e| AppError::io(&format!("报表生成异常: {}", e)))??;

    let path = save_file_dialog_async(&app, "HTML", &["html"], "设备报表.html").await?;
    let task_path = path.clone();
    tauri::async_runtime::spawn_blocking(move || {
        std::fs::write(&task_path, html)?;
        log::info!("[操作] 导出HTML报表: {:?}", task_path);
        Ok::<(), std::io::Error>(())
    })
    .await
    .map_err(|e| AppError::io(&format!("导出任务异常: {}", e)))??;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_excel_from_path(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    path: String,
    options: ImportOptions,
) -> Result<ImportResult, AppError> {
    log::info!("[操作] 开始导入Excel: {:?}, 模式={}, 关联机房={}", path, options.update_mode, options.link_room);
    let pool = state.pool.clone();
    let result = tauri::async_runtime::spawn_blocking(move || -> Result<ImportResult, AppError> {
        let conn = pool.get()?;
        crate::excel::import_devices_excel(&conn, &app, &path, &options)
    })
    .await
    .map_err(|e| AppError::io(&format!("导入任务异常: {}", e)))??;
    log::info!(
        "[操作] 导入Excel完成: 新增{}条, 更新{}条, 跳过{}条, 新建型号{}个, 告警{}条, 错误{}条",
        result.imported, result.updated, result.skipped, result.models_created,
        result.warnings.len(), result.errors.len()
    );
    Ok(result)
}
