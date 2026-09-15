use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use crate::models::*;
use crate::error::AppError;
use crate::db::{patch_assign, now_iso};

pub fn list_racks(conn: &Connection) -> Result<Vec<Rack>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, height_u, row, col, view, sort_order, room_id, created_at, updated_at FROM racks ORDER BY sort_order, id"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Rack {
            id: row.get(0)?,
            name: row.get(1)?,
            height_u: row.get(2)?,
            row: row.get(3)?,
            col: row.get(4)?,
            view: row.get(5)?,
            sort_order: row.get(6)?,
            room_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_rack(conn: &Connection, id: i32) -> Result<Option<Rack>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, height_u, row, col, view, sort_order, room_id, created_at, updated_at FROM racks WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Rack {
            id: row.get(0)?,
            name: row.get(1)?,
            height_u: row.get(2)?,
            row: row.get(3)?,
            col: row.get(4)?,
            view: row.get(5)?,
            sort_order: row.get(6)?,
            room_id: row.get(7)?,
            created_at: row.get(8)?,
            updated_at: row.get(9)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_rack(conn: &Connection, data: &RackCreate) -> Result<Rack, AppError> {
    let now = now_iso();
    conn.execute(
        "INSERT INTO racks (name, height_u, row, col, view, sort_order, room_id, created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            data.name,
            data.height_u.unwrap_or(42),
            data.row.unwrap_or(0),
            data.col.unwrap_or(0),
            data.view.as_deref().unwrap_or("front"),
            data.sort_order.unwrap_or(0),
            data.room_id,
            now.as_str(),
            now.as_str(),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_rack(conn, id)?.ok_or_else(|| AppError::not_found("机柜"))
}

pub fn update_rack(conn: &Connection, id: i32, data: &RackUpdate) -> Result<Option<Rack>, AppError> {
    let existing = get_rack(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut assignments: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    patch_assign!(assignments, params, "name", &data.name);
    patch_assign!(assignments, params, "height_u", &data.height_u);
    patch_assign!(assignments, params, "row", &data.row);
    patch_assign!(assignments, params, "col", &data.col);
    patch_assign!(assignments, params, "view", &data.view);
    patch_assign!(assignments, params, "sort_order", &data.sort_order);
    patch_assign!(assignments, params, "room_id", &data.room_id);

    if assignments.is_empty() {
        return Ok(existing);
    }

    // 时间戳单一维护方（§8-1）：刷新 updated_at
    assignments.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_iso()));

    let sql = format!(
        "UPDATE racks SET {} WHERE id = ?{}",
        assignments.join(", "),
        params.len() + 1
    );
    params.push(Box::new(id));

    conn.execute(&sql, params.iter().map(|p| p.as_ref()).collect::<Vec<_>>().as_slice())?;
    get_rack(conn, id)
}

pub fn delete_rack(conn: &Connection, id: i32) -> Result<bool, AppError> {
    // 删除机柜时其设备整体下架：清除机柜关联与 U 位区间，
    // 固有高度保留在 devices.height_u，重上架不会退回 1U。
    conn.execute(
        "UPDATE devices SET rack_id = NULL, start_u = NULL, end_u = NULL WHERE rack_id = ?1",
        params![id],
    )?;
    let affected = conn.execute("DELETE FROM racks WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

/// 查询所有机柜，返回 id→(name, room_id) 的 HashMap
pub fn list_racks_map(conn: &Connection) -> Result<std::collections::HashMap<i32, (String, Option<i32>)>, AppError> {
    let racks = list_racks(conn)?;
    Ok(racks.into_iter().map(|r| (r.id, (r.name, r.room_id))).collect())
}

/// 查询所有机房，返回 id→name 的 HashMap
pub fn list_rooms_map(conn: &Connection) -> Result<std::collections::HashMap<i32, String>, AppError> {
    let rooms = crate::db::rooms::list_rooms(conn)?;
    Ok(rooms.into_iter().map(|r| (r.id, r.name)).collect())
}

pub fn find_or_create_rack(conn: &Connection, name: &str) -> Result<i32, AppError> {
    // 并发安全：先查，未命中再插入；若其它事务已抢先插入同名机柜，
    // 唯一约束冲突会触发 ConstraintViolation，此时重查返回既有 id。
    // （配合调用方的 BEGIN IMMEDIATE 写事务，杜绝 SELECT→INSERT 竞态窗口）
    if let Some(id) = find_rack_id_by_name(conn, name)? {
        return Ok(id);
    }
    match conn.execute(
        "INSERT INTO racks (name, height_u, created_at, updated_at) VALUES (?1, 42, ?2, ?2)",
        params![name, now_iso()],
    ) {
        Ok(_) => Ok(conn.last_insert_rowid() as i32),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            find_rack_id_by_name(conn, name)?
                .ok_or_else(|| AppError::conflict(&format!("机柜「{}」创建冲突", name)))
        }
        Err(e) => Err(e.into()),
    }
}

fn find_rack_id_by_name(conn: &Connection, name: &str) -> Result<Option<i32>, AppError> {
    use rusqlite::OptionalExtension;
    conn.query_row("SELECT id FROM racks WHERE name = ?1", params![name], |row| row.get(0))
        .optional()
        .map_err(Into::into)
}

/// 导入关联机房（N-05）：仅当机柜当前**未归属任何机房**（`room_id IS NULL`）时将其
/// 关联到给定机房，避免覆盖用户在机柜管理中的手工归属；已归属则保持原样（幂等、非破坏）。
pub fn link_rack_room(conn: &Connection, rack_id: i32, room_id: i32) -> Result<(), AppError> {
    conn.execute(
        "UPDATE racks SET room_id = ?1, updated_at = ?2 WHERE id = ?3 AND room_id IS NULL",
        params![room_id, now_iso(), rack_id],
    )?;
    Ok(())
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
        assert!(list_racks(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        insert_rack(&conn, &RackCreate { name: "A01".into(), ..Default::default() }).unwrap();
        let racks = list_racks(&conn).unwrap();
        assert_eq!(racks.len(), 1);
        assert_eq!(racks[0].name, "A01");
        assert_eq!(racks[0].height_u, 42);
    }

    #[test]
    fn test_find_or_create() {
        let conn = setup_db();
        let id1 = find_or_create_rack(&conn, "A01").unwrap();
        let id2 = find_or_create_rack(&conn, "A01").unwrap();
        assert_eq!(id1, id2);
        let id3 = find_or_create_rack(&conn, "B01").unwrap();
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_delete_cascade() {
        let conn = setup_db();
        insert_rack(&conn, &RackCreate { name: "A01".into(), ..Default::default() }).unwrap();
        delete_rack(&conn, 1).unwrap();
        assert!(list_racks(&conn).unwrap().is_empty());
    }
}
