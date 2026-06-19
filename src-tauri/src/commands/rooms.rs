use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

#[tauri::command]
pub fn list_rooms(state: State<DbState>) -> Result<Vec<Room>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::rooms::list_rooms(&conn)
}

#[tauri::command]
pub fn get_room(state: State<DbState>, id: i32) -> Result<Option<Room>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    db::rooms::get_room(&conn, id)
}

#[tauri::command]
pub fn create_room(state: State<DbState>, data: RoomCreate) -> Result<Room, AppError> {
    if data.name.trim().is_empty() {
        return Err(AppError::validation("机房名称不能为空"));
    }
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::rooms::insert_room(c, &data))?;
    log::info!("[操作] 创建机房: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_room(state: State<DbState>, id: i32, data: RoomUpdate) -> Result<Option<Room>, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let result = db::with_transaction(&conn, |c| db::rooms::update_room(c, id, &data))?;
    if let Some(ref room) = result {
        log::info!("[操作] 更新机房: id={}, name={}", room.id, room.name);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_room(state: State<DbState>, id: i32) -> Result<bool, AppError> {
    let conn = state.pool.get().map_err(|e| AppError::io(&format!("连接池获取失败: {}", e)))?;
    let room_name = db::rooms::get_room(&conn, id)
        .ok()
        .flatten()
        .map(|r| r.name)
        .unwrap_or_default();
    let result = db::with_transaction(&conn, |c| db::rooms::delete_room(c, id))?;
    if result {
        log::info!("[操作] 删除机房: id={}, name={}", id, room_name);
    }
    Ok(result)
}
