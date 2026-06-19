use rusqlite::{Connection, params};
use chrono::NaiveDate;
use crate::models::*;
use crate::error::AppError;

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
    })
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
            conditions.push(format!("name LIKE ?{}", param_values.len() + 1));
            param_values.push(format!("%{}%", s));
        }
    }

    let sql = if conditions.is_empty() {
        String::from(
            "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
             ip_addresses, serial_no, asset_no, department, owner, \
             function, purchase_date, warranty_expire, status, power_watt \
             FROM devices ORDER BY name"
        )
    } else {
        format!(
            "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
             ip_addresses, serial_no, asset_no, department, owner, \
             function, purchase_date, warranty_expire, status, power_watt \
             FROM devices WHERE {} ORDER BY name",
            conditions.join(" AND ")
        )
    };

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params_refs.as_slice(), |row| row_to_device(row))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_device(conn: &Connection, id: i32) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, \
         function, purchase_date, warranty_expire, status, power_watt \
         FROM devices WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| row_to_device(row))?;
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

    conn.execute(
        "INSERT INTO devices (name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, function, \
         purchase_date, warranty_expire, status, power_watt) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
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
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_device(conn, id).map(|r| r.unwrap())
}

pub fn update_device(conn: &Connection, id: i32, data: &DeviceUpdate) -> Result<Option<Device>, AppError> {
    let existing = get_device(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }

    let purchase_date = parse_optional_date(&data.purchase_date);
    let warranty_expire = parse_optional_date(&data.warranty_expire);

    conn.execute(
        "UPDATE devices SET
            name = COALESCE(?1, name),
            device_model_id = COALESCE(?2, device_model_id),
            rack_id = COALESCE(?3, rack_id),
            start_u = COALESCE(?4, start_u),
            end_u = COALESCE(?5, end_u),
            ip_addresses = COALESCE(?6, ip_addresses),
            serial_no = COALESCE(?7, serial_no),
            asset_no = COALESCE(?8, asset_no),
            department = COALESCE(?9, department),
            owner = COALESCE(?10, owner),
            function = COALESCE(?11, function),
            purchase_date = COALESCE(?12, purchase_date),
            warranty_expire = COALESCE(?13, warranty_expire),
            status = COALESCE(?14, status),
            power_watt = COALESCE(?15, power_watt)
        WHERE id = ?16",
        params![
            data.name,
            data.device_model_id,
            data.rack_id,
            data.start_u,
            data.end_u,
            data.ip_addresses,
            data.serial_no,
            data.asset_no,
            data.department,
            data.owner,
            data.function,
            purchase_date,
            warranty_expire,
            data.status,
            data.power_watt,
            id,
        ],
    )?;
    get_device(conn, id)
}

pub fn find_device_by_serial(conn: &Connection, serial_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, \
         function, purchase_date, warranty_expire, status, power_watt \
         FROM devices WHERE serial_no = ?1 AND serial_no != ''"
    )?;
    let mut rows = stmt.query_map(params![serial_no], |row| row_to_device(row))?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_asset(conn: &Connection, asset_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, \
         function, purchase_date, warranty_expire, status, power_watt \
         FROM devices WHERE asset_no = ?1 AND asset_no != ''"
    )?;
    let mut rows = stmt.query_map(params![asset_no], |row| row_to_device(row))?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_name_in_rack(conn: &Connection, name: &str, rack_id: Option<i32>) -> Result<Option<Device>, AppError> {
    let result = match rack_id {
        Some(rid) => {
            let mut stmt = conn.prepare(
                "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
                 ip_addresses, serial_no, asset_no, department, owner, \
                 function, purchase_date, warranty_expire, status, power_watt \
                 FROM devices WHERE name = ?1 AND rack_id = ?2"
            )?;
            stmt.query_row(params![name, rid], |row| row_to_device(row)).ok()
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
                 ip_addresses, serial_no, asset_no, department, owner, \
                 function, purchase_date, warranty_expire, status, power_watt \
                 FROM devices WHERE name = ?1 AND rack_id IS NULL"
            )?;
            stmt.query_row(params![name], |row| row_to_device(row)).ok()
        }
    };
    Ok(result)
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
}
