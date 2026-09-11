use rusqlite::Connection;
use rusqlite::OptionalExtension;
use crate::error::AppError;

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    Ok(conn.query_row(
        "SELECT value FROM settings WHERE key = ?1",
        [key],
        |r| r.get(0),
    ).optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn is_logging_enabled(conn: &Connection) -> Result<bool, AppError> {
    let val = get_setting(conn, "logging_enabled")?;
    Ok(val.as_deref() == Some("true"))
}

pub fn set_logging_enabled(conn: &Connection, enabled: bool) -> Result<(), AppError> {
    set_setting(conn, "logging_enabled", if enabled { "true" } else { "false" })
}
