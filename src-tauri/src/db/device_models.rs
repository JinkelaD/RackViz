use rusqlite::{Connection, params};
use crate::models::*;
use crate::error::AppError;

pub fn list_device_models(conn: &Connection) -> Result<Vec<DeviceModel>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, manufacturer, type, height_u, power_watt FROM device_models ORDER BY name"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DeviceModel {
            id: row.get(0)?,
            name: row.get(1)?,
            manufacturer: row.get(2)?,
            device_type: row.get(3)?,
            height_u: row.get(4)?,
            power_watt: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_device_model(conn: &Connection, id: i32) -> Result<Option<DeviceModel>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, manufacturer, type, height_u, power_watt FROM device_models WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(DeviceModel {
            id: row.get(0)?,
            name: row.get(1)?,
            manufacturer: row.get(2)?,
            device_type: row.get(3)?,
            height_u: row.get(4)?,
            power_watt: row.get(5)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_device_model(conn: &Connection, data: &DeviceModelCreate) -> Result<DeviceModel, AppError> {
    conn.execute(
        "INSERT INTO device_models (name, manufacturer, type, height_u, power_watt) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            data.name,
            data.manufacturer.as_deref().unwrap_or(""),
            data.device_type.as_deref().unwrap_or("server"),
            data.height_u.unwrap_or(1),
            data.power_watt.unwrap_or(0),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_device_model(conn, id).map(|r| r.unwrap())
}

pub fn update_device_model(conn: &Connection, id: i32, data: &DeviceModelUpdate) -> Result<Option<DeviceModel>, AppError> {
    let existing = get_device_model(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }
    conn.execute(
        "UPDATE device_models SET
            name = COALESCE(?1, name),
            manufacturer = COALESCE(?2, manufacturer),
            type = COALESCE(?3, type),
            height_u = COALESCE(?4, height_u),
            power_watt = COALESCE(?5, power_watt)
        WHERE id = ?6",
        params![
            data.name, data.manufacturer, data.device_type,
            data.height_u, data.power_watt, id
        ],
    )?;
    get_device_model(conn, id)
}

pub fn delete_device_model(conn: &Connection, id: i32) -> Result<bool, AppError> {
    conn.execute("UPDATE devices SET device_model_id = NULL WHERE device_model_id = ?1", params![id])?;
    let affected = conn.execute("DELETE FROM device_models WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

/// 查询所有型号，返回 id→name 的 HashMap
pub fn list_models_map(conn: &Connection) -> Result<std::collections::HashMap<i32, String>, AppError> {
    let models = list_device_models(conn)?;
    Ok(models.into_iter().map(|m| (m.id, m.name)).collect())
}

pub fn find_or_create_model(conn: &Connection, name: &str) -> Result<i32, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM device_models WHERE name = ?1")?;
    let mut rows = stmt.query_map(params![name], |row| row.get::<_, i32>(0))?;
    if let Some(Ok(id)) = rows.next() {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO device_models (name, type) VALUES (?1, 'server')",
        params![name],
    )?;
    Ok(conn.last_insert_rowid() as i32)
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

    #[test]
    fn test_list_empty() {
        let conn = setup_db();
        assert!(list_device_models(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        insert_device_model(&conn, &DeviceModelCreate { name: "Dell R750".into(), ..Default::default() }).unwrap();
        let models = list_device_models(&conn).unwrap();
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].name, "Dell R750");
        assert_eq!(models[0].device_type, "server");
    }

    #[test]
    fn test_find_or_create() {
        let conn = setup_db();
        let id1 = find_or_create_model(&conn, "H3C S6520").unwrap();
        let id2 = find_or_create_model(&conn, "H3C S6520").unwrap();
        assert_eq!(id1, id2);
    }
}
