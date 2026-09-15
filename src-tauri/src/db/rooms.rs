use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use rusqlite::OptionalExtension;
use crate::models::*;
use crate::error::AppError;
use crate::db::{patch_assign, now_iso};

pub fn list_rooms(conn: &Connection) -> Result<Vec<Room>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, location, sort_order, created_at, updated_at FROM rooms ORDER BY sort_order, id"
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(Room {
            id: row.get(0)?,
            name: row.get(1)?,
            location: row.get(2)?,
            sort_order: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_room(conn: &Connection, id: i32) -> Result<Option<Room>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, location, sort_order, created_at, updated_at FROM rooms WHERE id = ?1"
    )?;
    let mut rows = stmt.query_map(params![id], |row| {
        Ok(Room {
            id: row.get(0)?,
            name: row.get(1)?,
            location: row.get(2)?,
            sort_order: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    Ok(rows.next().transpose()?)
}

pub fn insert_room(conn: &Connection, data: &RoomCreate) -> Result<Room, AppError> {
    let now = now_iso();
    conn.execute(
        "INSERT INTO rooms (name, location, sort_order, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            data.name,
            data.location.as_deref().unwrap_or(""),
            data.sort_order.unwrap_or(0),
            now.as_str(),
            now.as_str(),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_room(conn, id)?.ok_or_else(|| AppError::not_found("机房"))
}

pub fn update_room(conn: &Connection, id: i32, data: &RoomUpdate) -> Result<Option<Room>, AppError> {
    let existing = get_room(conn, id)?;
    if existing.is_none() {
        return Ok(None);
    }

    let mut assignments: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    patch_assign!(assignments, params, "name", &data.name);
    patch_assign!(assignments, params, "location", &data.location);
    patch_assign!(assignments, params, "sort_order", &data.sort_order);

    if assignments.is_empty() {
        return Ok(existing);
    }

    // 时间戳单一维护方（§8-1）：刷新 updated_at
    assignments.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_iso()));

    let sql = format!(
        "UPDATE rooms SET {} WHERE id = ?{}",
        assignments.join(", "),
        params.len() + 1
    );
    params.push(Box::new(id));

    conn.execute(&sql, params.iter().map(|p| p.as_ref()).collect::<Vec<_>>().as_slice())?;
    get_room(conn, id)
}

/// 按名称查找机房 id（幂等）。
#[allow(dead_code)] // 由 T2.5（excel 导入关联机房）间接消费
pub fn find_room_id_by_name(conn: &Connection, name: &str) -> Result<Option<i32>, AppError> {
    conn.query_row("SELECT id FROM rooms WHERE name = ?1", params![name], |row| row.get(0))
        .optional()
        .map_err(Into::into)
}

/// 按名称幂等查找或创建机房（N-05）。并发安全：先查，未命中再插入；
/// 若其它事务已抢先插入同名机房，唯一约束冲突时重查返回既有 id。
#[allow(dead_code)] // 由 T2.5（excel 导入关联机房）消费
pub fn find_or_create_room(conn: &Connection, name: &str) -> Result<i32, AppError> {
    if let Some(id) = find_room_id_by_name(conn, name)? {
        return Ok(id);
    }
    match conn.execute(
        "INSERT INTO rooms (name, location, sort_order, created_at, updated_at) VALUES (?1, '', 0, ?2, ?2)",
        params![name, now_iso()],
    ) {
        Ok(_) => Ok(conn.last_insert_rowid() as i32),
        Err(rusqlite::Error::SqliteFailure(e, _)) if e.code == rusqlite::ErrorCode::ConstraintViolation => {
            find_room_id_by_name(conn, name)?
                .ok_or_else(|| AppError::conflict(&format!("机房「{}」创建冲突", name)))
        }
        Err(e) => Err(e.into()),
    }
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
        let updated = update_room(&conn, 1, &RoomUpdate { name: Patch::Set("New".into()), ..Default::default() }).unwrap();
        assert_eq!(updated.unwrap().name, "New");
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();
        insert_room(&conn, &RoomCreate { name: "ToDelete".into(), ..Default::default() }).unwrap();
        assert!(delete_room(&conn, 1).unwrap());
        assert!(list_rooms(&conn).unwrap().is_empty());
    }

    #[test]
    fn test_find_or_create_room_idempotent() {
        let conn = setup_db();
        let id1 = find_or_create_room(&conn, "机房A").unwrap();
        let id2 = find_or_create_room(&conn, "机房A").unwrap();
        assert_eq!(id1, id2, "同名机房必须复用同一 id");
        let id3 = find_or_create_room(&conn, "机房B").unwrap();
        assert_ne!(id1, id3);
        assert_eq!(list_rooms(&conn).unwrap().len(), 2);
    }
}
