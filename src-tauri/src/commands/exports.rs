use tauri::State;
use crate::state::DbState;
use crate::error::AppError;
use crate::models::ImportResult;
use tauri_plugin_dialog::DialogExt;

fn save_path_to_string(path: Option<tauri_plugin_dialog::FilePath>) -> Result<std::path::PathBuf, AppError> {
    let fp = path.ok_or_else(|| AppError::cancelled("用户取消了保存"))?;
    match fp {
        tauri_plugin_dialog::FilePath::Path(p) => Ok(p),
        tauri_plugin_dialog::FilePath::Url(_) => Err(AppError::io("不支持URL路径")),
    }
}

#[tauri::command]
pub fn export_racks_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let data = {
        let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
        crate::excel::export_racks_excel(&conn)?
    };
    let path = app.dialog()
        .file()
        .add_filter("Excel", &["xlsx"])
        .set_file_name("机柜部署图.xlsx")
        .blocking_save_file();
    let path = save_path_to_string(path)?;
    std::fs::write(&path, data)?;
    log::info!("[操作] 导出机柜部署图: {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_devices_data_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let data = {
        let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
        crate::excel::export_devices_data_excel(&conn)?
    };
    let path = app.dialog()
        .file()
        .add_filter("Excel", &["xlsx"])
        .set_file_name("设备台账.xlsx")
        .blocking_save_file();
    let path = save_path_to_string(path)?;
    std::fs::write(&path, data)?;
    log::info!("[操作] 导出设备台账: {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_single_rack_excel(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
    rack_id: i32,
) -> Result<String, AppError> {
    let data = {
        let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
        crate::excel::export_single_rack_excel(&conn, rack_id)?
    };
    let path = app.dialog()
        .file()
        .add_filter("Excel", &["xlsx"])
        .set_file_name(format!("机柜_{}.xlsx", rack_id))
        .blocking_save_file();
    let path = save_path_to_string(path)?;
    std::fs::write(&path, data)?;
    log::info!("[操作] 导出单机柜Excel: rack_id={}, path={:?}", rack_id, path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn export_report_html(
    app: tauri::AppHandle,
    state: State<'_, DbState>,
) -> Result<String, AppError> {
    let html = {
        let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
        crate::report::render_report(&conn)?
    };
    let path = app.dialog()
        .file()
        .add_filter("HTML", &["html"])
        .set_file_name("设备报表.html")
        .blocking_save_file();
    let path = save_path_to_string(path)?;
    std::fs::write(&path, html)?;
    log::info!("[操作] 导出HTML报表: {:?}", path);
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn import_excel_from_path(
    state: State<DbState>,
    path: String,
) -> Result<ImportResult, AppError> {
    log::info!("[操作] 开始导入Excel: {:?}", path);
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = crate::excel::import_devices_excel(&conn, &path)?;
    log::info!("[操作] 导入Excel完成: 成功{}条, 跳过重复{}条, 错误{}条",
        result.imported, result.skipped, result.errors.len());
    Ok(result)
}
