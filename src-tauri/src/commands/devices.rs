use std::collections::HashSet;
use tauri::State;
use crate::state::DbState;
use crate::models::*;
use crate::db;
use crate::error::AppError;

/// 批量删除单次 IPC 的最大 id 数（N-20-5）。
const DELETE_BATCH_MAX: usize = 1000;
/// 批量删除日志最多记录的 id 个数（避免超长日志，N-20-5）。
const DELETE_BATCH_LOG_PREVIEW: usize = 20;

/// 分页查询设备台账（N-01/N-02）：服务端分页 + 多字段搜索 + 白名单排序。
#[tauri::command]
pub fn list_devices(state: State<DbState>, query: DeviceQuery) -> Result<DevicePage, AppError> {
    let conn = state.conn()?;
    let (items, total) = db::devices::query_devices(&conn, &query)?;
    Ok(DevicePage { items, total })
}

/// 全量列出设备（RackView / 导出 / 报表用，§3.3）。
#[tauri::command]
pub fn list_devices_all(state: State<DbState>) -> Result<Vec<Device>, AppError> {
    let conn = state.conn()?;
    db::devices::list_devices(&conn, None, None)
}

/// 回收站：列出已软删除设备（N-09）。
#[tauri::command]
pub fn list_deleted_devices(state: State<DbState>) -> Result<Vec<Device>, AppError> {
    let conn = state.conn()?;
    db::devices::list_deleted_devices(&conn, None)
}

/// 恢复软删除设备（N-09）：冲突预检 + 30 天窗口判定。
#[tauri::command]
pub fn restore_device(state: State<DbState>, id: i32) -> Result<Device, AppError> {
    let conn = state.conn()?;
    let dev = db::with_transaction(&conn, |c| db::devices::restore_device(c, id))?;
    log::info!("[操作] 恢复设备: id={}, name={}", dev.id, dev.name);
    Ok(dev)
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

/// 删除设备（N-09）：语义已由硬删改为**软删**，签名保持不变。
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
        log::info!("[操作] 删除设备(软删): id={}, name={}", id, dev_name);
    } else {
        log::warn!("[操作] 删除设备失败(未找到): id={}", id);
    }
    Ok(result)
}

/// 彻底删除（回收站永久清除，跳过 30 天保留期）：仅允许已软删记录物理 DELETE。
///
/// 返回被清除设备的名称（供前端提示）；设备不存在或未软删时报错。
/// 防误操作：前端强制两次确认；SQL 侧 `deleted_at IS NOT NULL` 兜底（active 设备不可能被命中）。
#[tauri::command]
pub fn purge_device(state: State<DbState>, id: i32) -> Result<String, AppError> {
    let conn = state.conn()?;
    let dev = db::devices::get_device(&conn, id)?
        .ok_or_else(|| AppError::not_found("设备"))?;
    if dev.deleted_at.is_none() {
        return Err(AppError::validation("该设备不在回收站中，无法彻底删除"));
    }
    let purged = db::with_transaction(&conn, |c| db::devices::purge_device(c, id))?;
    if purged {
        log::info!("[操作] 彻底删除设备(硬删): id={}, name={}", dev.id, dev.name);
    } else {
        log::warn!("[操作] 彻底删除设备失败(未找到或未软删): id={}", id);
        return Err(AppError::not_found("设备"));
    }
    Ok(dev.name)
}

/// 批量软删除设备（N-20）：单次 IPC；去重 + 上限校验；同事务循环复用单条软删。
#[tauri::command]
pub fn delete_devices(state: State<DbState>, ids: Vec<i32>) -> Result<DeleteBatchResult, AppError> {
    // ids 去重（保持首次出现顺序）
    let mut seen: HashSet<i32> = HashSet::new();
    let mut unique: Vec<i32> = Vec::with_capacity(ids.len());
    for id in ids {
        if seen.insert(id) {
            unique.push(id);
        }
    }

    // 空选区：直接返回，不开启事务
    if unique.is_empty() {
        return Ok(DeleteBatchResult { deleted: 0, not_found: Vec::new() });
    }

    // 上限校验
    if unique.len() > DELETE_BATCH_MAX {
        return Err(AppError::validation(&format!(
            "单次批量删除不能超过 {} 个设备（当前 {} 个）",
            DELETE_BATCH_MAX,
            unique.len()
        )));
    }

    let conn = state.conn()?;
    let (deleted, not_found) =
        db::with_transaction(&conn, |c| db::devices::delete_devices_batch(c, &unique))?;

    // 日志截断：只记「共 N 个 id，前 20 个: [...]」
    let preview: Vec<i32> = unique.iter().take(DELETE_BATCH_LOG_PREVIEW).copied().collect();
    log::info!(
        "[操作] 批量删除设备: 共 {} 个 id, 前 {} 个: {:?}, 成功 {} 台, 跳过 {} 个",
        unique.len(),
        preview.len(),
        preview,
        deleted,
        not_found.len()
    );

    Ok(DeleteBatchResult { deleted, not_found })
}
