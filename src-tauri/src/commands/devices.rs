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
    let conn = state.conn()?;
    db::devices::list_devices(&conn, rack_id, search)
}

#[tauri::command]
pub fn get_device(state: State<DbState>, id: i32) -> Result<Option<Device>, AppError> {
    let conn = state.conn()?;
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
    let conn = state.conn()?;
    let result = db::with_transaction(&conn, |c| db::devices::insert_device(c, &data))?;
    log::info!("[操作] 创建设备: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_device(state: State<DbState>, id: i32, data: DeviceUpdate) -> Result<Option<Device>, AppError> {
    let conn = state.conn()?;
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
    let conn = state.conn()?;
    // 先查询设备名称用于日志（DB 错误记录但不阻断删除）
    let dev_name = match db::devices::get_device(&conn, id) {
        Ok(Some(d)) => d.name,
        Ok(None) => String::new(),
        Err(e) => { log::warn!("删除设备前查询名称失败: {}", e); String::new() }
    };
    let result = db::with_transaction(&conn, |c| db::devices::delete_device(c, id))?;
    if result {
        log::info!("[操作] 删除设备: id={}, name={}", id, dev_name);
    } else {
        log::warn!("[操作] 删除设备失败(未找到): id={}", id);
    }
    Ok(result)
}
