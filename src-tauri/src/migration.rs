use rusqlite::Connection;

const CURRENT_VERSION: i32 = 2;

pub fn run(conn: &Connection) -> Result<(), rusqlite::Error> {
    let version: i32 = conn
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap_or(0);

    match version {
        0 => { migrate_v0_to_v1(conn)?; migrate_v1_to_v2(conn)?; }
        1 => migrate_v1_to_v2(conn)?,
        2 => {}
        _ => {
            log::warn!("数据库版本 {} 高于当前版本 {}", version, CURRENT_VERSION);
        }
    }
    Ok(())
}

fn migrate_v0_to_v1(conn: &Connection) -> Result<(), rusqlite::Error> {
    let has_data: bool = conn
        .query_row("SELECT COUNT(*) > 0 FROM rooms", [], |r| r.get(0))
        .unwrap_or(false);

    if has_data {
        conn.execute_batch(
            "UPDATE racks SET room_id = NULL WHERE room_id IS NOT NULL AND room_id NOT IN (SELECT id FROM rooms);
             UPDATE devices SET rack_id = NULL WHERE rack_id IS NOT NULL AND rack_id NOT IN (SELECT id FROM racks);
             UPDATE devices SET device_model_id = NULL WHERE device_model_id IS NOT NULL AND device_model_id NOT IN (SELECT id FROM device_models);"
        )?;
    }

    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS rooms (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            location    TEXT DEFAULT '',
            sort_order  INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS racks (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            height_u    INTEGER NOT NULL DEFAULT 42,
            row         INTEGER DEFAULT 0,
            col         INTEGER DEFAULT 0,
            view        TEXT DEFAULT 'front',
            sort_order  INTEGER DEFAULT 0,
            room_id     INTEGER REFERENCES rooms(id) ON DELETE SET NULL
        );

        CREATE TABLE IF NOT EXISTS device_models (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            name        TEXT NOT NULL,
            manufacturer TEXT DEFAULT '',
            type        TEXT NOT NULL DEFAULT 'server',
            height_u    INTEGER NOT NULL DEFAULT 1,
            power_watt  INTEGER DEFAULT 0
        );

        CREATE TABLE IF NOT EXISTS devices (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            name            TEXT NOT NULL,
            device_model_id INTEGER REFERENCES device_models(id) ON DELETE SET NULL,
            rack_id         INTEGER REFERENCES racks(id) ON DELETE SET NULL,
            start_u         INTEGER,
            end_u           INTEGER,
            ip_addresses    TEXT DEFAULT '',
            serial_no       TEXT DEFAULT '',
            asset_no        TEXT DEFAULT '',
            department      TEXT DEFAULT '',
            owner           TEXT DEFAULT '',
            function        TEXT DEFAULT '',
            purchase_date   DATE,
            warranty_expire DATE,
            status          TEXT DEFAULT 'unconfigured',
            power_watt      INTEGER DEFAULT 0
        );

        PRAGMA user_version = 1;"
    )?;

    Ok(())
}

fn migrate_v1_to_v2(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS settings (
            key     TEXT PRIMARY KEY,
            value   TEXT NOT NULL
        );
        INSERT OR IGNORE INTO settings (key, value) VALUES ('logging_enabled', 'false');
        PRAGMA user_version = 2;"
    )?;
    Ok(())
}
