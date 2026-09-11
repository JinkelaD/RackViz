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
    pub fn new(db_path: &std::path::Path, app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error>> {
        let db_str = db_path.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "数据库路径包含非法字符")
        })?;
        let manager = SqliteConnectionManager::file(db_str);
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

    /// 从连接池取一条连接。
    /// 依赖 `From<r2d2::Error> for AppError` 自动转换，避免各处重复 `map_err(AppError::io)` 误分类。
    pub fn conn(&self) -> Result<r2d2::PooledConnection<SqliteConnectionManager>, AppError> {
        Ok(self.pool.get()?)
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
