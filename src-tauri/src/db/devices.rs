use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use rusqlite::OptionalExtension;
use chrono::{DateTime, NaiveDate, Utc};
use crate::models::*;
use crate::error::AppError;
use crate::db::{patch_assign, NOT_DELETED, now_iso};

/// 转义 LIKE 模式中的通配符，防止用户输入 `%`/`_`/`\` 干扰匹配。
/// SQL 侧必须配套 `ESCAPE '\'`。
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn row_to_device(row: &rusqlite::Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: row.get(0)?,
        name: row.get(1)?,
        device_model_id: row.get(2)?,
        rack_id: row.get(3)?,
        start_u: row.get(4)?,
        end_u: row.get(5)?,
        ip_addresses: row.get::<_, String>(6).unwrap_or_default(),
        serial_no: row.get::<_, String>(7).unwrap_or_default(),
        asset_no: row.get::<_, String>(8).unwrap_or_default(),
        department: row.get::<_, String>(9).unwrap_or_default(),
        owner: row.get::<_, String>(10).unwrap_or_default(),
        function: row.get::<_, String>(11).unwrap_or_default(),
        purchase_date: row.get(12)?,
        warranty_expire: row.get(13)?,
        status: row.get::<_, String>(14).unwrap_or_else(|_| "unconfigured".into()),
        power_watt: row.get(15).unwrap_or(0),
        height_u: row.get::<_, i32>(16).unwrap_or(1),
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        deleted_at: row.get(19)?,
    })
}

const DEVICE_SELECT: &str =
    "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
     ip_addresses, serial_no, asset_no, department, owner, \
     function, purchase_date, warranty_expire, status, power_watt, height_u, \
     created_at, updated_at, deleted_at \
     FROM devices";

/// 推导设备固有高度：显式值 > 已给 U 位区间 > 型号高度 > 1。
fn resolve_device_height(
    conn: &Connection,
    explicit: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
    device_model_id: Option<i32>,
) -> Result<i32, AppError> {
    if let Some(h) = explicit {
        if h >= 1 {
            return Ok(h);
        }
    }
    if let (Some(s), Some(e)) = (start_u, end_u) {
        if e >= s {
            return Ok(e - s + 1);
        }
    }
    if let Some(mid) = device_model_id {
        let h: Option<i32> = conn
            .query_row("SELECT height_u FROM device_models WHERE id = ?1", params![mid], |r| r.get(0))
            .optional()?;
        if let Some(h) = h.filter(|h| *h >= 1) {
            return Ok(h);
        }
    }
    Ok(1)
}

/// 回收站可恢复窗口（天）：超过该天数的软删记录拒绝恢复（§9 Q6，固定常量，不物理清理）。
const RESTORE_WINDOW_DAYS: i64 = 30;

/// `query_devices` 分页默认/上限（防止前端传超大 limit 拖垮 UI）。
const QUERY_DEFAULT_LIMIT: i64 = 100;
const QUERY_MAX_LIMIT: i64 = 1000;

/// 全量列出设备（RackView / 导出 / 报表用）。
///
/// **保留全量语义**：恒追加 `deleted_at IS NULL`（§10.2 第 8 项裁决，不暴露 `include_deleted`）。
pub fn list_devices(conn: &Connection, rack_id: Option<i32>, search: Option<String>) -> Result<Vec<Device>, AppError> {
    let mut conditions: Vec<String> = vec![NOT_DELETED.to_string()];
    let mut param_values: Vec<String> = Vec::new();

    if let Some(rid) = rack_id {
        conditions.push(format!("rack_id = ?{}", param_values.len() + 1));
        param_values.push(rid.to_string());
    }
    if let Some(ref s) = search {
        if !s.trim().is_empty() {
            // 通配符转义 + ESCAPE '\'（审查红线 S-2 / D-1）
            conditions.push(format!("name LIKE ?{} ESCAPE '\\'", param_values.len() + 1));
            param_values.push(format!("%{}%", escape_like(s)));
        }
    }

    let sql = format!(
        "{} WHERE {} ORDER BY name",
        DEVICE_SELECT,
        conditions.join(" AND ")
    );

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params_refs.as_slice(), row_to_device)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

/// 服务端分页 + 多字段搜索 + 白名单排序（N-01/N-02）。
///
/// 返回 `(items, total)`：`total` 为**同一 WHERE 条件下**的 `COUNT(*)`。
///
/// - 过滤：默认 `deleted_at IS NULL`；`include_deleted=true` 时才放行；
///   `rack_id = ?`；`room_id` 经 `rack_id IN (SELECT id FROM racks WHERE room_id = ?)`。
/// - 搜索：`(name LIKE ? OR ip_addresses LIKE ? OR serial_no LIKE ? OR asset_no LIKE ?)`，
///   复用 `escape_like` 并带 `ESCAPE '\'`。
/// - 排序：**白名单映射**字段 → 列名；`sort_order` 仅 `asc|desc`；**绝不允许前端字符串裸拼接**。
/// - 分页：`LIMIT ? OFFSET ?`。
pub fn query_devices(conn: &Connection, q: &DeviceQuery) -> Result<(Vec<Device>, i64), AppError> {
    let include_deleted = q.include_deleted.unwrap_or(false);

    let mut where_parts: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    // 1) 软删除过滤（默认排除）
    if !include_deleted {
        where_parts.push(NOT_DELETED.to_string());
    }
    // 2) 机柜过滤
    if let Some(rid) = q.rack_id {
        params.push(Box::new(rid));
        where_parts.push(format!("rack_id = ?{}", params.len()));
    }
    // 3) 机房过滤（经机柜表）
    if let Some(room_id) = q.room_id {
        params.push(Box::new(room_id));
        where_parts.push(format!(
            "rack_id IN (SELECT id FROM racks WHERE room_id = ?{})",
            params.len()
        ));
    }
    // 4) 多字段搜索（四字段同参）
    if let Some(ref s) = q.search {
        let s = s.trim();
        if !s.is_empty() {
            params.push(Box::new(format!("%{}%", escape_like(s))));
            let idx = params.len();
            where_parts.push(format!(
                "(name LIKE ?{idx} ESCAPE '\\' \
                 OR ip_addresses LIKE ?{idx} ESCAPE '\\' \
                 OR serial_no LIKE ?{idx} ESCAPE '\\' \
                 OR asset_no LIKE ?{idx} ESCAPE '\\')"
            ));
        }
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_parts.join(" AND "))
    };

    // total：同 WHERE 的 COUNT(*)
    let count_sql = format!("SELECT COUNT(*) FROM devices{}", where_clause);
    let count_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let total: i64 = conn.query_row(&count_sql, count_refs.as_slice(), |r| r.get(0))?;

    // 排序白名单（仅映射，不做任何未校验拼接）
    let order_col = match q.sort_field.as_deref() {
        Some("ip_addresses") => "ip_addresses",
        Some("serial_no") => "serial_no",
        Some("asset_no") => "asset_no",
        Some("status") => "status",
        Some("power_watt") => "power_watt",
        Some("created_at") => "created_at",
        Some("updated_at") => "updated_at",
        Some("rack_id") => "rack_id",
        Some("name") => "name",
        _ => "name",
    };
    let order_dir = match q.sort_order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    // 分页参数（夹取到安全范围）
    let limit = q.limit.unwrap_or(QUERY_DEFAULT_LIMIT).clamp(1, QUERY_MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);

    let list_sql = format!(
        "{}{} ORDER BY {} {} LIMIT ?{} OFFSET ?{}",
        DEVICE_SELECT,
        where_clause,
        order_col,
        order_dir,
        params.len() + 1,
        params.len() + 2
    );

    let mut stmt = conn.prepare(&list_sql)?;
    let mut list_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    list_refs.push(&limit);
    list_refs.push(&offset);
    let rows = stmt.query_map(list_refs.as_slice(), row_to_device)?;
    let items = rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)?;

    Ok((items, total))
}

/// 回收站列表：仅软删除记录，按 `deleted_at DESC`（N-09）。
pub fn list_deleted_devices(conn: &Connection, limit: Option<i64>) -> Result<Vec<Device>, AppError> {
    let limit = limit.unwrap_or(500).clamp(1, 5000);
    let sql = format!(
        "{} WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC LIMIT ?1",
        DEVICE_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], row_to_device)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_device(conn: &Connection, id: i32) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!("{} WHERE id = ?1", DEVICE_SELECT))?;
    let mut rows = stmt.query_map(params![id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

fn parse_optional_date(s: &Option<String>) -> Option<NaiveDate> {
    s.as_deref().and_then(|v| {
        if v.trim().is_empty() { return None; }
        NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(v.trim(), "%Y/%m/%d"))
            .or_else(|_| NaiveDate::parse_from_str(v.trim(), "%Y.%m.%d"))
            .ok()
    })
}

pub fn insert_device(conn: &Connection, data: &DeviceCreate) -> Result<Device, AppError> {
    let purchase_date = parse_optional_date(&data.purchase_date);
    let warranty_expire = parse_optional_date(&data.warranty_expire);
    let height_u = resolve_device_height(conn, data.height_u, data.start_u, data.end_u, data.device_model_id)?;
    // 时间戳单一维护方（§8-1）：insert 时 created_at = updated_at = now
    let now = now_iso();

    conn.execute(
        "INSERT INTO devices (name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, function, \
         purchase_date, warranty_expire, status, power_watt, height_u, \
         created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            data.name,
            data.device_model_id,
            data.rack_id,
            data.start_u,
            data.end_u,
            data.ip_addresses.as_deref().unwrap_or(""),
            data.serial_no.as_deref().unwrap_or(""),
            data.asset_no.as_deref().unwrap_or(""),
            data.department.as_deref().unwrap_or(""),
            data.owner.as_deref().unwrap_or(""),
            data.function.as_deref().unwrap_or(""),
            purchase_date,
            warranty_expire,
            data.status.as_deref().unwrap_or("unconfigured"),
            data.power_watt.unwrap_or(0),
            height_u,
            now.as_str(),
            now.as_str(),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))
}

pub fn update_device(conn: &Connection, id: i32, data: &DeviceUpdate) -> Result<Option<Device>, AppError> {
    let existing = get_device(conn, id)?;
    let Some(cur) = existing else {
        return Ok(None);
    };

    // 显式字段列表动态 SQL：Unset 跳过、Set 赋值、Clear 置 NULL（替代 COALESCE）
    let mut assignments: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    patch_assign!(assignments, params, "name", &data.name);
    patch_assign!(assignments, params, "device_model_id", &data.device_model_id);
    patch_assign!(assignments, params, "rack_id", &data.rack_id);
    patch_assign!(assignments, params, "start_u", &data.start_u);
    patch_assign!(assignments, params, "end_u", &data.end_u);
    patch_assign!(assignments, params, "ip_addresses", &data.ip_addresses);
    patch_assign!(assignments, params, "serial_no", &data.serial_no);
    patch_assign!(assignments, params, "asset_no", &data.asset_no);
    patch_assign!(assignments, params, "department", &data.department);
    patch_assign!(assignments, params, "owner", &data.owner);
    patch_assign!(assignments, params, "function", &data.function);
    patch_assign!(assignments, params, "status", &data.status);
    patch_assign!(assignments, params, "power_watt", &data.power_watt);

    // 日期字段：值转 NaiveDate 存储；Clear 或空串 → NULL
    push_date_assignment(&mut assignments, &mut params, "purchase_date", &data.purchase_date)?;
    push_date_assignment(&mut assignments, &mut params, "warranty_expire", &data.warranty_expire)?;

    // 固有高度 height_u（NOT NULL，不允许 Clear）：
    // - Set(v) → 显式写值（v ≥ 1，否则报校验错）；
    // - Unset → 若本次更新使设备落在有效 U 位区间（start/end 皆非空且 end≥start），
    //   自动同步为区间高度（重上架/换位置时保证高度不丢）；未落在区间则保持原值（下架保留高度）。
    match &data.height_u {
        Patch::Clear => {
            return Err(AppError::validation("设备固有高度不能置空"));
        }
        Patch::Set(v) if *v < 1 => {
            return Err(AppError::validation(&format!("设备固有高度无效: {}（须 ≥ 1）", v)));
        }
        Patch::Set(v) => {
            assignments.push(format!("height_u = ?{}", params.len() + 1));
            params.push(Box::new(*v));
        }
        Patch::Unset => {
            // 结合既有行计算更新后的实际 U 位
            let eff_start = match &data.start_u {
                Patch::Set(v) => Some(*v),
                Patch::Clear => None,
                Patch::Unset => cur.start_u,
            };
            let eff_end = match &data.end_u {
                Patch::Set(v) => Some(*v),
                Patch::Clear => None,
                Patch::Unset => cur.end_u,
            };
            if let (Some(s), Some(e)) = (eff_start, eff_end) {
                if e >= s && e - s + 1 != cur.height_u {
                    assignments.push(format!("height_u = ?{}", params.len() + 1));
                    params.push(Box::new(e - s + 1));
                }
            }
        }
    }

    if assignments.is_empty() {
        return Ok(Some(cur));
    }

    // 时间戳单一维护方（§8-1）：任何更新刷新 updated_at = now
    assignments.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_iso()));

    let sql = format!(
        "UPDATE devices SET {} WHERE id = ?{}",
        assignments.join(", "),
        params.len() + 1
    );
    params.push(Box::new(id));

    conn.execute(&sql, params.iter().map(|p| p.as_ref()).collect::<Vec<_>>().as_slice())?;
    get_device(conn, id)
}

/// 将 Patch<String> 日期写入参数列表：Set(非空可解析) → NaiveDate；
/// Set(空串) / Clear → NULL；Unset → 忽略。
fn push_date_assignment(
    assignments: &mut Vec<String>,
    params: &mut Vec<Box<dyn ToSql>>,
    column: &str,
    patch: &Patch<String>,
) -> Result<(), AppError> {
    match patch {
        Patch::Unset => {}
        Patch::Clear => assignments.push(format!("{} = NULL", column)),
        Patch::Set(s) if s.trim().is_empty() => assignments.push(format!("{} = NULL", column)),
        Patch::Set(s) => match parse_optional_date(&Some(s.clone())) {
            Some(date) => {
                assignments.push(format!("{} = ?{}", column, params.len() + 1));
                params.push(Box::new(date));
            }
            None => {
                return Err(AppError::validation(&format!(
                    "{} 日期格式无效: {}（支持 YYYY-MM-DD / YYYY/MM/DD / YYYY.MM.DD）",
                    column, s
                )));
            }
        },
    }
    Ok(())
}

pub fn find_device_by_serial(conn: &Connection, serial_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE serial_no = ?1 AND serial_no != '' AND {}",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![serial_no], row_to_device)?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_asset(conn: &Connection, asset_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE asset_no = ?1 AND asset_no != '' AND {}",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![asset_no], row_to_device)?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_name_in_rack(conn: &Connection, name: &str, rack_id: Option<i32>) -> Result<Option<Device>, AppError> {
    match rack_id {
        Some(rid) => {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE name = ?1 AND rack_id = ?2",
                DEVICE_SELECT
            ))?;
            Ok(stmt.query_row(params![name, rid], row_to_device).optional()?)
        }
        None => {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE name = ?1 AND rack_id IS NULL",
                DEVICE_SELECT
            ))?;
            Ok(stmt.query_row(params![name], row_to_device).optional()?)
        }
    }
}

/// 单条软删除（N-09）。`delete_device` 的语义实现；批量删除亦复用它（§8-13）。
///
/// `UPDATE devices SET deleted_at = now, updated_at = now WHERE id = ? AND deleted_at IS NULL`，
/// 返回受影响行数是否 > 0。
pub fn soft_delete_device(conn: &Connection, id: i32) -> Result<bool, AppError> {
    let now = now_iso();
    let affected = conn.execute(
        "UPDATE devices SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        params![now.as_str(), now.as_str(), id],
    )?;
    Ok(affected > 0)
}

/// 删除设备（对外语义）：按 §10.1 改为**软删除**，内部转调 `soft_delete_device`。
pub fn delete_device(conn: &Connection, id: i32) -> Result<bool, AppError> {
    soft_delete_device(conn, id)
}

/// 批量软删除（N-20）：在**同一事务**内循环复用单条软删（§8-13，禁止 `DELETE ... IN`）。
///
/// 返回 `(deleted 成功数, not_found 不存在 / 已被软删的 id 列表)`。
/// 事务的 `BEGIN/COMMIT/ROLLBACK` 由调用方（命令层）通过 `with_transaction` 包裹。
pub fn delete_devices_batch(conn: &Connection, ids: &[i32]) -> Result<(u32, Vec<i32>), AppError> {
    let mut deleted: u32 = 0;
    let mut not_found: Vec<i32> = Vec::new();
    for &id in ids {
        if soft_delete_device(conn, id)? {
            deleted += 1;
        } else {
            not_found.push(id);
        }
    }
    Ok((deleted, not_found))
}

/// 解析 ISO8601 UTC 时间戳（`%Y-%m-%dT%H:%M:%SZ`），失败返回 None。
fn parse_iso_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// 冲突预检：在**其它 active 设备**中查找占用同一 `serial_no` 的记录（排除自身）。
fn find_active_by_serial_excluding(
    conn: &Connection,
    serial_no: &str,
    exclude_id: i32,
) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE serial_no = ?1 AND serial_no != '' AND {} AND id != ?2",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![serial_no, exclude_id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

/// 冲突预检：在**其它 active 设备**中查找占用同一 `asset_no` 的记录（排除自身）。
fn find_active_by_asset_excluding(
    conn: &Connection,
    asset_no: &str,
    exclude_id: i32,
) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE asset_no = ?1 AND asset_no != '' AND {} AND id != ?2",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![asset_no, exclude_id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

/// 恢复软删除设备（N-09）：
/// ① 读取该行 `deleted_at`，**超 30 天拒绝恢复**；
/// ② **冲突预检**——`serial_no` / `asset_no` 是否被其它 active 设备占用，命中则
///    `AppError::conflict("恢复失败：序列号 SN-x 已被设备「Y」占用，请先处理该设备")`，**不自动改名**；
/// ③ 通过则 `UPDATE devices SET deleted_at = NULL, updated_at = now WHERE id = ?` 并返回该 `Device`。
pub fn restore_device(conn: &Connection, id: i32) -> Result<Device, AppError> {
    let dev = get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))?;

    // ① 必须处于软删状态
    let deleted_at = dev
        .deleted_at
        .clone()
        .ok_or_else(|| AppError::validation("该设备未被删除，无需恢复"))?;

    // ① 超 30 天拒绝恢复
    if let Some(dt) = parse_iso_utc(&deleted_at) {
        let age_days = (Utc::now() - dt).num_days();
        if age_days > RESTORE_WINDOW_DAYS {
            return Err(AppError::validation(&format!(
                "恢复失败：设备删除已超过 {} 天，无法恢复",
                RESTORE_WINDOW_DAYS
            )));
        }
    }

    // ② 冲突预检（序列号）
    if !dev.serial_no.trim().is_empty() {
        if let Some(other) = find_active_by_serial_excluding(conn, &dev.serial_no, dev.id)? {
            return Err(AppError::conflict(&format!(
                "恢复失败：序列号 {} 已被设备「{}」占用，请先处理该设备",
                dev.serial_no, other.name
            )));
        }
    }
    // ② 冲突预检（资产编号）
    if !dev.asset_no.trim().is_empty() {
        if let Some(other) = find_active_by_asset_excluding(conn, &dev.asset_no, dev.id)? {
            return Err(AppError::conflict(&format!(
                "恢复失败：资产编号 {} 已被设备「{}」占用，请先处理该设备",
                dev.asset_no, other.name
            )));
        }
    }

    // ③ 恢复
    conn.execute(
        "UPDATE devices SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
        params![now_iso(), id],
    )?;
    get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::migration;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        migration::run(&conn).unwrap();
        conn
    }

    fn create_test_device(conn: &Connection, name: &str, rack_id: Option<i32>) -> Device {
        if let Some(rid) = rack_id {
            // 确保机柜存在（满足外键约束）
            if crate::db::racks::get_rack(conn, rid).unwrap().is_none() {
                crate::db::racks::insert_rack(conn, &crate::models::RackCreate { name: format!("TestRack{}", rid), ..Default::default() }).unwrap();
            }
        }
        insert_device(conn, &DeviceCreate {
            name: name.into(),
            rack_id,
            ..Default::default()
        }).unwrap()
    }

    #[test]
    fn test_list_empty() {
        let conn = setup_db();
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        create_test_device(&conn, "Web Server", None);
        let devices = list_devices(&conn, None, None).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Web Server");
    }

    #[test]
    fn test_filter_by_rack() {
        let conn = setup_db();
        create_test_device(&conn, "Dev1", Some(1));
        create_test_device(&conn, "Dev2", Some(2));
        let devices = list_devices(&conn, Some(1), None).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Dev1");
    }

    #[test]
    fn test_search() {
        let conn = setup_db();
        create_test_device(&conn, "Core Router", None);
        create_test_device(&conn, "Access Switch", None);
        let results = list_devices(&conn, None, Some("Router".into())).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Core Router");
    }

    #[test]
    fn test_search_like_wildcard_escaped() {
        // 回归：搜索 `100%` 不得匹配名称含 100 但不含 % 的设备（LIKE 通配符转义）
        let conn = setup_db();
        create_test_device(&conn, "Rate100", None);
        create_test_device(&conn, "Rate100%Gold", None);
        // 转义后 `%100\%%` 只匹配字面 "100%"
        let results = list_devices(&conn, None, Some("100%".into())).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Rate100%Gold");

        // `_` 同理：`A_C` 不应匹配 "ABC"
        let conn2 = setup_db();
        create_test_device(&conn2, "ABC", None);
        create_test_device(&conn2, "A_C", None);
        let results2 = list_devices(&conn2, None, Some("A_C".into())).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].name, "A_C");
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();
        create_test_device(&conn, "ToDelete", None);
        assert!(delete_device(&conn, 1).unwrap());
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent() {
        let conn = setup_db();
        assert!(!delete_device(&conn, 999).unwrap());
    }

    /// 上架一台设备（分配到机柜 1 的 U5）
    fn mount_device(conn: &Connection, name: &str) -> Device {
        // 确保机柜 1 存在（满足外键约束）
        if crate::db::racks::get_rack(conn, 1).unwrap().is_none() {
            crate::db::racks::insert_rack(conn, &crate::models::RackCreate {
                name: "MountRack".into(),
                ..Default::default()
            }).unwrap();
        }
        insert_device(conn, &DeviceCreate {
            name: name.into(),
            rack_id: Some(1),
            start_u: Some(5),
            end_u: Some(7),
            status: Some("offline".into()),
            ..Default::default()
        }).unwrap()
    }

    #[test]
    fn test_insert_derives_height_from_u_span() {
        // 无型号设备：插入时高度由 start_u/end_u 区间推导
        let conn = setup_db();
        let dev = mount_device(&conn, "MultiU");
        assert_eq!(dev.start_u, Some(5));
        assert_eq!(dev.end_u, Some(7));
        assert_eq!(dev.height_u, 3);
    }

    #[test]
    fn test_height_persists_after_unrack_and_rerack() {
        // 回归：P0 — 多U设备下架后重上架退化为 1U
        let conn = setup_db();
        let dev = mount_device(&conn, "MultiU");
        assert_eq!(dev.height_u, 3);

        // 1) 下架：清空机柜与 U 位，height_u 必须保留
        let unracked = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Clear,
            start_u: Patch::Clear,
            end_u: Patch::Clear,
            status: Patch::Set("unconfigured".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(unracked.rack_id, None);
        assert_eq!(unracked.height_u, 3, "下架后固有高度不得丢失");

        // 2) 重上架到新位置（只传机柜与 U 位，模拟拖拽）
        let racked = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Set(1),
            start_u: Patch::Set(2),
            end_u: Patch::Set(4),
            status: Patch::Set("offline".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(racked.start_u, Some(2));
        assert_eq!(racked.end_u, Some(4));
        assert_eq!(racked.height_u, 3, "重上架后仍应为 3U");
    }

    #[test]
    fn test_update_clear_racks_off_shelf() {
        // 回归：P0 — COALESCE 无法置空，导致"拖回资源池/未上架"失效
        let conn = setup_db();
        let dev = mount_device(&conn, "ToUnmount");
        assert_eq!(dev.rack_id, Some(1));

        // 模拟 RackView.handleStockDrop / DeviceDetailPanel「未上架」：显式传 null
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Clear,
            start_u: Patch::Clear,
            end_u: Patch::Clear,
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.rack_id, None);
        assert_eq!(updated.start_u, None);
        assert_eq!(updated.end_u, None);
        assert_eq!(updated.status, "offline"); // 其它字段不受影响
    }

    #[test]
    fn test_update_partial_keeps_other_columns() {
        let conn = setup_db();
        let dev = mount_device(&conn, "PartialUpdate");
        // 只更新 status（在线），rack/u 位必须保持不变
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            status: Patch::Set("online".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.status, "online");
        assert_eq!(updated.rack_id, Some(1));
        assert_eq!(updated.start_u, Some(5));
        assert_eq!(updated.end_u, Some(7));
    }

    #[test]
    fn test_update_set_field_values() {
        let conn = setup_db();
        let dev = mount_device(&conn, "Rename");
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            name: Patch::Set("Renamed".into()),
            serial_no: Patch::Set("SN-001".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.serial_no, "SN-001");
        assert_eq!(updated.rack_id, Some(1)); // 未提及字段保持
    }

    #[test]
    fn test_update_clear_text_and_date() {
        let conn = setup_db();
        let dev = mount_device(&conn, "ClearFields");
        // 文本置空字符串 + 日期 Clear → NULL
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            serial_no: Patch::Set("".into()),
            purchase_date: Patch::Clear,
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.serial_no, "");
        assert_eq!(updated.purchase_date, None);
    }

    #[test]
    fn test_update_unknown_id_returns_none() {
        let conn = setup_db();
        let result = update_device(&conn, 999, &DeviceUpdate {
            name: Patch::Set("X".into()),
            ..Default::default()
        }).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_date_invalid_format() {
        let conn = setup_db();
        let dev = mount_device(&conn, "BadDate");
        let result = update_device(&conn, dev.id, &DeviceUpdate {
            purchase_date: Patch::Set("2026-13-99".into()),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    // ==================== N-01/N-02 query_devices ====================

    #[test]
    fn test_query_devices_excludes_deleted_by_default() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Alive", None);
        let gone = create_test_device(&conn, "Gone", None);
        soft_delete_device(&conn, gone.id).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery::default()).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, dev.id);

        // include_deleted = true 时全部返回
        let (items2, total2) = query_devices(&conn, &DeviceQuery {
            include_deleted: Some(true),
            ..Default::default()
        }).unwrap();
        assert_eq!(total2, 2);
        assert_eq!(items2.len(), 2);
    }

    #[test]
    fn test_query_devices_multi_field_search() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "TOK-Name".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "B".into(), ip_addresses: Some("TOK-ip".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "C".into(), serial_no: Some("TOK-SN".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "D".into(), asset_no: Some("TOK-AS".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Nope".into(), ..Default::default() }).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery {
            search: Some("TOK".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 4);
        assert_eq!(items.len(), 4);
    }

    #[test]
    fn test_query_devices_sort_whitelist() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "Zeta".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Alpha".into(), ..Default::default() }).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery {
            sort_field: Some("name".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items[0].name, "Zeta");
        assert_eq!(items[1].name, "Alpha");

        // 非白名单字段回退 name（不得报错、不得注入）
        let (items_fallback, _) = query_devices(&conn, &DeviceQuery {
            sort_field: Some("name; DROP TABLE devices".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(items_fallback.len(), 2);
        assert_eq!(items_fallback[0].name, "Zeta");
    }

    #[test]
    fn test_query_devices_pagination_and_filter() {
        let conn = setup_db();
        for i in 1..=5 {
            create_test_device(&conn, &format!("Dev{}", i), Some(1));
        }
        create_test_device(&conn, "Other", Some(2));

        // 机柜过滤 + 分页
        let (page1, total) = query_devices(&conn, &DeviceQuery {
            rack_id: Some(1),
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 5);
        assert_eq!(page1.len(), 2);

        let (page2, _) = query_devices(&conn, &DeviceQuery {
            rack_id: Some(1),
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        }).unwrap();
        assert_eq!(page2.len(), 2);
        // 两页不重叠
        assert_ne!(page1[0].id, page2[0].id);
    }

    // ==================== N-09 soft delete / restore ====================

    #[test]
    fn test_soft_delete_hides_and_supports_recreate_same_serial() {
        let conn = setup_db();
        let first = insert_device(&conn, &DeviceCreate {
            name: "Old".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        }).unwrap();

        assert!(soft_delete_device(&conn, first.id).unwrap());
        // 默认不可见
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
        // 回收站可见
        let deleted = list_deleted_devices(&conn, None).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, first.id);

        // 同序列号可重建（部分唯一索引生效）
        let second = insert_device(&conn, &DeviceCreate {
            name: "New".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        }).unwrap();
        assert_ne!(first.id, second.id);

        // active 之间仍强唯一
        let dup = insert_device(&conn, &DeviceCreate {
            name: "Dup".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        });
        assert!(dup.is_err(), "active 重复序列号必须被拒");
    }

    #[test]
    fn test_soft_delete_device_idempotent_returns_false() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Once", None);
        assert!(soft_delete_device(&conn, dev.id).unwrap());
        // 再次软删：已删（deleted_at 非 NULL）→ 0 行
        assert!(!soft_delete_device(&conn, dev.id).unwrap());
        // 不存在的 id
        assert!(!soft_delete_device(&conn, 9999).unwrap());
    }

    #[test]
    fn test_restore_device_success() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "RestoreMe", None);
        soft_delete_device(&conn, dev.id).unwrap();
        let restored = restore_device(&conn, dev.id).unwrap();
        assert_eq!(restored.id, dev.id);
        assert!(restored.deleted_at.is_none());
        assert_eq!(list_devices(&conn, None, None).unwrap().len(), 1);
    }

    #[test]
    fn test_restore_conflict_returns_conflict() {
        use crate::error::ErrorCode;
        let conn = setup_db();
        let old = insert_device(&conn, &DeviceCreate {
            name: "Old".into(),
            serial_no: Some("SN-C".into()),
            ..Default::default()
        }).unwrap();
        soft_delete_device(&conn, old.id).unwrap();

        // 软删期间用同序列号建了新设备
        insert_device(&conn, &DeviceCreate {
            name: "New".into(),
            serial_no: Some("SN-C".into()),
            ..Default::default()
        }).unwrap();

        let err = restore_device(&conn, old.id).unwrap_err();
        match err {
            AppError { code: ErrorCode::Conflict, .. } => {}
            other => panic!("期望 Conflict，实际: {:?}", other),
        }
    }

    #[test]
    fn test_restore_over_30_days_rejected() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "TooOld", None);
        soft_delete_device(&conn, dev.id).unwrap();

        // 将 deleted_at 回拨到 31 天前
        let past = (chrono::Utc::now() - chrono::Duration::days(31))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        conn.execute(
            "UPDATE devices SET deleted_at = ?1 WHERE id = ?2",
            params![past, dev.id],
        ).unwrap();

        let err = restore_device(&conn, dev.id);
        assert!(err.is_err(), "超过 30 天必须拒绝恢复");
    }

    #[test]
    fn test_restore_active_device_rejected() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Active", None);
        // 未软删直接恢复 → 拒绝
        assert!(restore_device(&conn, dev.id).is_err());
    }

    // ==================== N-20 delete_devices_batch ====================

    #[test]
    fn test_delete_devices_batch_counts_and_not_found() {
        let conn = setup_db();
        let a = create_test_device(&conn, "A", None);
        let b = create_test_device(&conn, "B", None);

        let (deleted, not_found) = delete_devices_batch(&conn, &[a.id, b.id, 9999]).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(not_found, vec![9999]);
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
        assert_eq!(list_deleted_devices(&conn, None).unwrap().len(), 2);
    }

    #[test]
    fn test_delete_devices_batch_already_deleted_goes_to_not_found() {
        let conn = setup_db();
        let a = create_test_device(&conn, "A", None);
        soft_delete_device(&conn, a.id).unwrap();

        let (deleted, not_found) = delete_devices_batch(&conn, &[a.id]).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(not_found, vec![a.id]);
    }
}
