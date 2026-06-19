use rusqlite::{Connection, params};
use crate::models::*;
use crate::error::AppError;

pub fn list_rooms(conn: &Connection) -> Result<Vec<Room>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, location, sort_order FROM rooms ORDER BY sort_order, id"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Room {
            id: row.get(0)?,
            name: row.get(1)?,
            location: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_room(conn: &Connection, id: i32) -> Result<Option<Room>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, location, sort_order FROM rooms WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Room {
            id: row.get(0)?,
            name: row.get(1)?,
            location: row.get(2)?,
            sort_order: row.get(3)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_room(conn: &Connection, data: &RoomCreate) -> Result<Room, AppError> {
    conn.execute(
        "INSERT INTO rooms (name, location, sort_order) VALUES (?1, ?2, ?3)",
        params![
            data.name,
            data.location.as_deref().unwrap_or(""),
            data.sort_order.unwrap_or(0),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_room(conn, id).map(|r| r.unwrap())
}

pub fn update_room(conn: &Connection, id: i32, data: &RoomUpdate) -> Result<Option<Room>, AppError> {
    let existing = get_room(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }
    conn.execute(
        "UPDATE rooms SET name = COALESCE(?1, name), location = COALESCE(?2, location), sort_order = COALESCE(?3, sort_order) WHERE id = ?4",
        params![data.name, data.location, data.sort_order, id],
    )?;
    get_room(conn, id)
}

pub fn delete_room(conn: &Connection, id: i32) -> Result<bool, AppError> {
    conn.execute("UPDATE racks SET room_id = NULL WHERE room_id = ?1", params![id])?;
    let affected = conn.execute("DELETE FROM rooms WHERE id = ?1", params![id])?;
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

    #[test]
    fn test_list_empty() {
        let conn = setup_db();
        let rooms = list_rooms(&conn).unwrap();
        assert!(rooms.is_empty());
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        insert_room(&conn, &RoomCreate { name: "A机房".into(), ..Default::default() }).unwrap();
        let rooms = list_rooms(&conn).unwrap();
        assert_eq!(rooms.len(), 1);
        assert_eq!(rooms[0].name, "A机房");
    }

    #[test]
    fn test_get_nonexistent() {
        let conn = setup_db();
        assert!(get_room(&conn, 999).unwrap().is_none());
    }

    #[test]
    fn test_update() {
        let conn = setup_db();
        insert_room(&conn, &RoomCreate { name: "Old".into(), ..Default::default() }).unwrap();
        let updated = update_room(&conn, 1, &RoomUpdate { name: Some("New".into()), ..Default::default() }).unwrap();
        assert_eq!(updated.unwrap().name, "New");
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();
        insert_room(&conn, &RoomCreate { name: "ToDelete".into(), ..Default::default() }).unwrap();
        assert!(delete_room(&conn, 1).unwrap());
        assert!(list_rooms(&conn).unwrap().is_empty());
    }
}
