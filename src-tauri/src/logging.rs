use std::path::PathBuf;
use std::sync::Mutex;
use flexi_logger::{Logger, FileSpec, Age, Naming, Cleanup, Criterion, LogSpecBuilder, Duplicate, WriteMode};
use log::LevelFilter;

/// 全局持有的 flexi_logger 句柄，用于运行时切换日志级别
static LOGGER_HANDLE: Mutex<Option<flexi_logger::LoggerHandle>> = Mutex::new(None);

/// 获取日志句柄锁（容忍 poison：日志系统不应因其它线程 panic 而崩溃）
fn lock_logger() -> std::sync::MutexGuard<'static, Option<flexi_logger::LoggerHandle>> {
    LOGGER_HANDLE.lock().unwrap_or_else(|e| e.into_inner())
}

/// 获取日志目录路径
pub fn log_dir(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("logs")
}

/// 初始化日志系统
/// - enabled=true: 写入文件 + stderr 输出 Info 级别
/// - enabled=false: 仅 stderr 输出 Warn 级别（几乎静默），不写文件
pub fn init(app_data_dir: &std::path::Path, enabled: bool) -> Result<(), Box<dyn std::error::Error>> {
    let log_path = log_dir(app_data_dir);
    std::fs::create_dir_all(&log_path)?;

    if enabled {
        let spec = LogSpecBuilder::new()
            .default(LevelFilter::Info)
            .build();

        let handle = Logger::with(spec)
            .log_to_file(
                FileSpec::default()
                    .directory(&log_path)
                    .basename("rackviz")
                    .suppress_timestamp()
                    .suffix("log"),
            )
            .duplicate_to_stderr(Duplicate::All)
            .write_mode(WriteMode::BufferAndFlush)
            .rotate(
                Criterion::Age(Age::Day),
                Naming::Timestamps,
                Cleanup::KeepLogFiles(7),
            )
            .format_for_files(flexi_logger::detailed_format)
            .format_for_stderr(flexi_logger::colored_detailed_format)
            .start()?;

        *lock_logger() = Some(handle);
    } else {
        // 关闭时：只输出 Warn 到 stderr，不写文件
        let spec = LogSpecBuilder::new()
            .default(LevelFilter::Warn)
            .build();

        let handle = Logger::with(spec)
            .log_to_stderr()
            .format_for_stderr(flexi_logger::colored_detailed_format)
            .start()?;

        *lock_logger() = Some(handle);
    }

    Ok(())
}

/// 运行时动态切换日志开关
pub fn reconfigure(app: tauri::AppHandle, enabled: bool) -> Result<(), crate::error::AppError> {
    use tauri::Manager;

    let app_data_dir = app.path().app_local_data_dir()
        .map_err(|e| crate::error::AppError::io(&format!("无法获取应用数据目录: {}", e)))?;
    let log_path = log_dir(&app_data_dir);
    std::fs::create_dir_all(&log_path)?;

    if enabled {
        let spec = LogSpecBuilder::new()
            .default(LevelFilter::Info)
            .build();

        let mut guard = lock_logger();
        if let Some(handle) = guard.as_mut() {
            handle.set_new_spec(spec);
        }
        log::info!("日志收集已开启");
    } else {
        let spec = LogSpecBuilder::new()
            .default(LevelFilter::Warn)
            .build();

        let mut guard = lock_logger();
        if let Some(handle) = guard.as_mut() {
            handle.set_new_spec(spec);
        }
        // 注意：关闭日志后这条 warn 不会写文件，但会输出到 stderr
        log::warn!("日志收集已关闭");
    }

    Ok(())
}
