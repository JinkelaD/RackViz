use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

#[tauri::command]
pub fn list_device_models(state: State<DbState>) -> Result<Vec<DeviceModel>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::device_models::list_device_models(&conn)
}

#[tauri::command]
pub fn get_device_model(state: State<DbState>, id: i32) -> Result<Option<DeviceModel>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::device_models::get_device_model(&conn, id)
}

#[tauri::command]
pub fn create_device_model(state: State<DbState>, data: DeviceModelCreate) -> Result<DeviceModel, AppError> {
    if data.name.trim().is_empty() {
        return Err(AppError::validation("型号名称不能为空"));
    }
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::device_models::insert_device_model(c, &data))?;
    log::info!("[操作] 创建设备型号: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_device_model(state: State<DbState>, id: i32, data: DeviceModelUpdate) -> Result<Option<DeviceModel>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::device_models::update_device_model(c, id, &data))?;
    if let Some(ref model) = result {
        log::info!("[操作] 更新设备型号: id={}, name={}", model.id, model.name);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_device_model(state: State<DbState>, id: i32) -> Result<bool, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let model_name = db::device_models::get_device_model(&conn, id)
        .ok()
        .flatten()
        .map(|m| m.name)
        .unwrap_or_default();
    let result = db::with_transaction(&conn, |c| db::device_models::delete_device_model(c, id))?;
    if result {
        log::info!("[操作] 删除设备型号: id={}, name={}", id, model_name);
    }
    Ok(result)
}
