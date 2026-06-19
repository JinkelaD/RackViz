use rusqlite::{Connection, params};
use crate::models::*;
use crate::error::AppError;

pub fn list_racks(conn: &Connection) -> Result<Vec<Rack>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, height_u, row, col, view, sort_order, room_id FROM racks ORDER BY sort_order, id"
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
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_rack(conn: &Connection, id: i32) -> Result<Option<Rack>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, height_u, row, col, view, sort_order, room_id FROM racks WHERE id = ?1"
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
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_rack(conn: &Connection, data: &RackCreate) -> Result<Rack, AppError> {
    conn.execute(
        "INSERT INTO racks (name, height_u, row, col, view, sort_order, room_id) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            data.name,
            data.height_u.unwrap_or(42),
            data.row.unwrap_or(0),
            data.col.unwrap_or(0),
            data.view.as_deref().unwrap_or("front"),
            data.sort_order.unwrap_or(0),
            data.room_id,
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_rack(conn, id).map(|r| r.unwrap())
}

pub fn update_rack(conn: &Connection, id: i32, data: &RackUpdate) -> Result<Option<Rack>, AppError> {
    let existing = get_rack(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }
    conn.execute(
        "UPDATE racks SET
            name = COALESCE(?1, name),
            height_u = COALESCE(?2, height_u),
            row = COALESCE(?3, row),
            col = COALESCE(?4, col),
            view = COALESCE(?5, view),
            sort_order = COALESCE(?6, sort_order),
            room_id = COALESCE(?7, room_id)
        WHERE id = ?8",
        params![
            data.name, data.height_u, data.row, data.col,
            data.view, data.sort_order, data.room_id, id
        ],
    )?;
    get_rack(conn, id)
}

pub fn delete_rack(conn: &Connection, id: i32) -> Result<bool, AppError> {
    conn.execute("UPDATE devices SET rack_id = NULL WHERE rack_id = ?1", params![id])?;
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
    let mut stmt = conn.prepare("SELECT id FROM racks WHERE name = ?1")?;
    let mut rows = stmt.query_map(params![name], |row| row.get::<_, i32>(0))?;
    if let Some(Ok(id)) = rows.next() {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO racks (name, height_u) VALUES (?1, 42)",
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
