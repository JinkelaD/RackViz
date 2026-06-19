use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

#[tauri::command]
pub fn list_racks(state: State<DbState>) -> Result<Vec<Rack>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::racks::list_racks(&conn)
}

#[tauri::command]
pub fn get_rack(state: State<DbState>, id: i32) -> Result<Option<Rack>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::racks::get_rack(&conn, id)
}

#[tauri::command]
pub fn create_rack(state: State<DbState>, data: RackCreate) -> Result<Rack, AppError> {
    if data.name.trim().is_empty() {
        return Err(AppError::validation("机柜名称不能为空"));
    }
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::racks::insert_rack(c, &data))?;
    log::info!("[操作] 创建机柜: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_rack(state: State<DbState>, id: i32, data: RackUpdate) -> Result<Option<Rack>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::racks::update_rack(c, id, &data))?;
    if let Some(ref rack) = result {
        log::info!("[操作] 更新机柜: id={}, name={}", rack.id, rack.name);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_rack(state: State<DbState>, id: i32) -> Result<bool, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let rack_name = db::racks::get_rack(&conn, id)
        .ok()
        .flatten()
        .map(|r| r.name)
        .unwrap_or_default();
    let result = db::with_transaction(&conn, |c| db::racks::delete_rack(c, id))?;
    if result {
        log::info!("[操作] 删除机柜: id={}, name={}", id, rack_name);
    }
    Ok(result)
}
