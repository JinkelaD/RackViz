use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

#[tauri::command]
pub fn list_devices(
    state: State<DbState>,
    rack_id: Option<i32>,
    search: Option<String>,
) -> Result<Vec<Device>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::devices::list_devices(&conn, rack_id, search)
}

#[tauri::command]
pub fn get_device(state: State<DbState>, id: i32) -> Result<Option<Device>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::devices::get_device(&conn, id)
}

#[tauri::command]
pub fn create_device(state: State<DbState>, data: DeviceCreate) -> Result<Device, AppError> {
    if data.name.trim().is_empty() {
        return Err(AppError::validation("设备名称不能为空"));
    }
    if data.name.len() > 100 {
        return Err(AppError::validation("设备名称不能超过100个字符"));
    }
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::devices::insert_device(c, &data))?;
    log::info!("[操作] 创建设备: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_device(state: State<DbState>, id: i32, data: DeviceUpdate) -> Result<Option<Device>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::devices::update_device(c, id, &data))?;
    if let Some(ref dev) = result {
        log::info!("[操作] 更新设备: id={}, name={}", dev.id, dev.name);
    } else {
        log::warn!("[操作] 更新设备失败(未找到): id={}", id);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_device(state: State<DbState>, id: i32) -> Result<bool, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    // 先查询设备名称用于日志
    let dev_name = db::devices::get_device(&conn, id)
        .ok()
        .flatten()
        .map(|d| d.name)
        .unwrap_or_default();
    let result = db::with_transaction(&conn, |c| db::devices::delete_device(c, id))?;
    if result {
        log::info!("[操作] 删除设备: id={}, name={}", id, dev_name);
    } else {
        log::warn!("[操作] 删除设备失败(未找到): id={}", id);
    }
    Ok(result)
}
