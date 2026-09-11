use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

#[tauri::command]
pub fn list_rooms(state: State<DbState>) -> Result<Vec<Room>, AppError> {
    let conn = state.conn()?;
    db::rooms::list_rooms(&conn)
}

#[tauri::command]
pub fn get_room(state: State<DbState>, id: i32) -> Result<Option<Room>, AppError> {
    let conn = state.conn()?;
    db::rooms::get_room(&conn, id)
}

#[tauri::command]
pub fn create_room(state: State<DbState>, data: RoomCreate) -> Result<Room, AppError> {
    if data.name.trim().is_empty() {
        return Err(AppError::validation("机房名称不能为空"));
    }
    let conn = state.conn()?;
    let result = db::with_transaction(&conn, |c| db::rooms::insert_room(c, &data))?;
    log::info!("[操作] 创建机房: id={}, name={}", result.id, result.name);
    Ok(result)
}

#[tauri::command]
pub fn update_room(state: State<DbState>, id: i32, data: RoomUpdate) -> Result<Option<Room>, AppError> {
    let conn = state.conn()?;
    let result = db::with_transaction(&conn, |c| db::rooms::update_room(c, id, &data))?;
    if let Some(ref room) = result {
        log::info!("[操作] 更新机房: id={}, name={}", room.id, room.name);
    }
    Ok(result)
}

#[tauri::command]
pub fn delete_room(state: State<DbState>, id: i32) -> Result<bool, AppError> {
    let conn = state.conn()?;
    // 先查询机房名称用于日志（DB 错误记录但不阻断删除）
    let room_name = match db::rooms::get_room(&conn, id) {
        Ok(Some(r)) => r.name,
        Ok(None) => String::new(),
        Err(e) => { log::warn!("删除机房前查询名称失败: {}", e); String::new() }
    };
    let result = db::with_transaction(&conn, |c| db::rooms::delete_room(c, id))?;
    if result {
        log::info!("[操作] 删除机房: id={}, name={}", id, room_name);
    }
    Ok(result)
}
