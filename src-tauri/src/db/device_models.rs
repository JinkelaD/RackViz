use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use crate::models::*;
use crate::error::AppError;
use crate::db::{patch_assign, now_iso};

pub fn list_device_models(conn: &Connection) -> Result<Vec<DeviceModel>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, manufacturer, type, height_u, power_watt, created_at, updated_at FROM device_models ORDER BY name"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(DeviceModel {
            id: row.get(0)?,
            name: row.get(1)?,
            manufacturer: row.get(2)?,
            device_type: row.get(3)?,
            height_u: row.get(4)?,
            power_watt: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_device_model(conn: &Connection, id: i32) -> Result<Option<DeviceModel>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, manufacturer, type, height_u, power_watt, created_at, updated_at FROM device_models WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(DeviceModel {
            id: row.get(0)?,
            name: row.get(1)?,
            manufacturer: row.get(2)?,
            device_type: row.get(3)?,
            height_u: row.get(4)?,
            power_watt: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_device_model(conn: &Connection, data: &DeviceModelCreate) -> Result<DeviceModel, AppError> {
    let now = now_iso();
    conn.execute(
        "INSERT INTO device_models (name, manufacturer, type, height_u, power_watt, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            data.name,
            data.manufacturer.as_deref().unwrap_or(""),
            data.device_type.as_deref().unwrap_or("server"),
            data.height_u.unwrap_or(1),
            data.power_watt.unwrap_or(0),
            now.as_str(),
            now.as_str(),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_device_model(conn, id)?.ok_or_else(|| AppError::not_found("型号"))
}

pub fn update_device_model(conn: &Connection, id: i32, data: &DeviceModelUpdate) -> Result<Option<DeviceModel>, AppError> {
    let existing = get_device_model(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut assignments: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    patch_assign!(assignments, params, "name", &data.name);
    patch_assign!(assignments, params, "manufacturer", &data.manufacturer);
    patch_assign!(assignments, params, "type", &data.device_type);
    patch_assign!(assignments, params, "height_u", &data.height_u);
    patch_assign!(assignments, params, "power_watt", &data.power_watt);

    if assignments.is_empty() {
        return Ok(existing);
    }

    // 时间戳单一维护方（§8-1）：刷新 updated_at
    assignments.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_iso()));

    let sql = format!(
        "UPDATE device_models SET {} WHERE id = ?{}",
        assignments.join(", "),
        params.len() + 1
    );
    params.push(Box::new(id));

    conn.execute(&sql, params.iter().map(|p| p.as_ref()).collect::<Vec<_>>().as_slice())?;
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

/// 按名称幂等查找或创建型号（N-21 签名扩展）。
///
/// - 新建型号使用传入 `device_type`（`None` → `'server'` 兜底）；
/// - **同名型号已存在 → 复用其 id，且绝不覆盖既有 `type`**（避免覆盖用户在型号管理里的手工修正）；
/// - 并发安全：先查，未命中再插入；唯一约束冲突时重查返回既有 id。
pub fn find_or_create_model(conn: &Connection, name: &str, device_type: Option<&str>) -> Result<i32, AppError> {
    if let Some(id) = find_model_id_by_name(conn, name)? {
        return Ok(id);
    }
    let ty = device_type.unwrap_or("server");
    let now = now_iso();
    match conn.execute(
        "INSERT INTO device_models (name, type, created_at, updated_at) VALUES (?1, ?2, ?3, ?3)",
        params![name, ty, now],
    ) {
        Ok(_) => Ok(conn.last_insert_rowid() as i32),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            find_model_id_by_name(conn, name)?
                .ok_or_else(|| AppError::conflict(&format!("型号「{}」创建冲突", name)))
        }
        Err(e) => Err(e.into()),
    }
}

fn find_model_id_by_name(conn: &Connection, name: &str) -> Result<Option<i32>, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row("SELECT id FROM device_models WHERE name = ?1", params![name], |row| row.get(0))
        .optional()
        .map_err(Into::into)
}

/// 按名称查询完整型号（N-21）：供 Excel 导入时判断「同名型号已存在则复用且不覆盖 type」，
/// 并据既有 `type` 生成告警文案（"型号「X」已存在，类型保持为 Y"）。
pub fn get_device_model_by_name(conn: &Connection, name: &str) -> Result<Option<DeviceModel>, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row(
        "SELECT id, name, manufacturer, type, height_u, power_watt, created_at, updated_at \
         FROM device_models WHERE name = ?1",
        params![name],
        |row| {
            Ok(DeviceModel {
                id: row.get(0)?,
                name: row.get(1)?,
                manufacturer: row.get(2)?,
                device_type: row.get(3)?,
                height_u: row.get(4)?,
                power_watt: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
            })
        },
    )
    .optional()
    .map_err(Into::into)
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
        let id1 = find_or_create_model(&conn, "H3C S6520", None).unwrap();
        let id2 = find_or_create_model(&conn, "H3C S6520", None).unwrap();
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_find_or_create_model_uses_device_type() {
        let conn = setup_db();
        let id = find_or_create_model(&conn, "Cisco C9300", Some("switch")).unwrap();
        let model = get_device_model(&conn, id).unwrap().unwrap();
        assert_eq!(model.device_type, "switch");
    }

    #[test]
    fn test_find_or_create_model_none_falls_back_to_server() {
        let conn = setup_db();
        let id = find_or_create_model(&conn, "Generic", None).unwrap();
        let model = get_device_model(&conn, id).unwrap().unwrap();
        assert_eq!(model.device_type, "server");
    }

    #[test]
    fn test_find_or_create_model_does_not_overwrite_existing_type() {
        let conn = setup_db();
        // 用户手工修正为 router
        let id = find_or_create_model(&conn, "Edge", Some("router")).unwrap();
        // 再次导入同名型号、传不同类型 → 复用 id，且不覆盖既有 type
        let id2 = find_or_create_model(&conn, "Edge", Some("switch")).unwrap();
        assert_eq!(id, id2);
        let model = get_device_model(&conn, id).unwrap().unwrap();
        assert_eq!(model.device_type, "router", "同名型号既有 type 不得被覆盖");
    }
}
