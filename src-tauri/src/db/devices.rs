use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use rusqlite::OptionalExtension;
use chrono::NaiveDate;
use crate::models::*;
use crate::error::AppError;
use crate::db::patch_assign;

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
    })
}

const DEVICE_SELECT: &str =
    "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
     ip_addresses, serial_no, asset_no, department, owner, \
     function, purchase_date, warranty_expire, status, power_watt, height_u \
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

pub fn list_devices(conn: &Connection, rack_id: Option<i32>, search: Option<String>) -> Result<Vec<Device>, AppError> {
    let mut conditions: Vec<String> = Vec::new();
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

    let sql = if conditions.is_empty() {
        format!("{} ORDER BY name", DEVICE_SELECT)
    } else {
        format!(
            "{} WHERE {} ORDER BY name",
            DEVICE_SELECT,
            conditions.join(" AND ")
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params_refs.as_slice(), row_to_device)?;
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

    conn.execute(
        "INSERT INTO devices (name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, function, \
         purchase_date, warranty_expire, status, power_watt, height_u) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
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
    let mut stmt = conn.prepare(&format!("{} WHERE serial_no = ?1 AND serial_no != ''", DEVICE_SELECT))?;
    let mut rows = stmt.query_map(params![serial_no], row_to_device)?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_asset(conn: &Connection, asset_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!("{} WHERE asset_no = ?1 AND asset_no != ''", DEVICE_SELECT))?;
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

pub fn delete_device(conn: &Connection, id: i32) -> Result<bool, AppError> {
    let affected = conn.execute("DELETE FROM devices WHERE id = ?1", params![id])?;
    Ok(affected > 0)
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
}
