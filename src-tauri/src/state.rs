use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Connection;
use tauri::AppHandle;
use tauri::Manager;
use crate::migration;
use crate::error::AppError;

pub type DbPool = Pool<SqliteConnectionManager>;

/// 自定义连接初始化器 — 确保每次从池中取出连接时 PRAGMA 都生效
#[derive(Debug)]
struct RackVizConnectionCustomizer;

impl r2d2::CustomizeConnection<Connection, rusqlite::Error> for RackVizConnectionCustomizer {
    fn on_acquire(&self, conn: &mut Connection) -> Result<(), rusqlite::Error> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA busy_timeout=5000;"
        )?;
        Ok(())
    }
}

pub struct DbState {
    pub pool: DbPool,
    app_handle: AppHandle,
}

impl DbState {
    pub fn new(db_path: &str, app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let manager = SqliteConnectionManager::file(db_path);
        let pool = Pool::builder()
            .max_size(4)
            .connection_customizer(Box::new(RackVizConnectionCustomizer))
            .build(manager)?;

        // 执行迁移（用一个临时连接）
        let conn = pool.get()?;
        migration::run(&conn)?;

        Ok(Self { pool, app_handle })
    }

    /// 获取日志目录路径
    pub fn log_dir(&self) -> std::path::PathBuf {
        let app_dir = self.app_handle.path().app_local_data_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."));
        crate::logging::log_dir(&app_dir)
    }

    #[allow(dead_code)]
    pub fn app_handle(&self) -> &AppHandle {
        &self.app_handle
    }
}

/// 事务包装器 — 自动 BEGIN IMMEDIATE / COMMIT / ROLLBACK
pub fn with_transaction<F, T>(conn: &Connection, f: F) -> Result<T, AppError>
where
    F: FnOnce(&Connection) -> Result<T, AppError>,
{
    conn.execute_batch("BEGIN IMMEDIATE")?;
    match f(conn) {
        Ok(result) => {
            conn.execute_batch("COMMIT")?;
            Ok(result)
        }
        Err(e) => {
            let _ = conn.execute_batch("ROLLBACK");
            Err(e)
        }
    }
}
