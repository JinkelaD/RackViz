mod error;
mod models;
mod migration;
mod state;
mod db;
mod commands;
mod excel;
mod report;
mod logging;

use state::DbState;
use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_dir = app.path().app_local_data_dir()?;
            std::fs::create_dir_all(&app_dir)?;
            let db_path = app_dir.join("rackviz.db");

            // 先初始化数据库，读取 logging 配置
            let db_state = DbState::new(&db_path, app.handle().clone())?;

            let logging_enabled = {
                let conn = db_state.pool.get()?;
                crate::db::settings::is_logging_enabled(&conn).unwrap_or(false)
            };

            // 初始化日志系统
            crate::logging::init(&app_dir, logging_enabled)?;

            log::info!("数据库路径: {:?}", db_path);
            log::info!("日志收集: {}", if logging_enabled { "已开启" } else { "已关闭" });

            app.manage(db_state);
            log::info!("RackViz 启动完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::devices::list_devices,
            commands::devices::get_device,
            commands::devices::create_device,
            commands::devices::update_device,
            commands::devices::delete_device,
            commands::racks::list_racks,
            commands::racks::get_rack,
            commands::racks::create_rack,
            commands::racks::update_rack,
            commands::racks::delete_rack,
            commands::rooms::list_rooms,
            commands::rooms::get_room,
            commands::rooms::create_room,
            commands::rooms::update_room,
            commands::rooms::delete_room,
            commands::device_models::list_device_models,
            commands::device_models::get_device_model,
            commands::device_models::create_device_model,
            commands::device_models::update_device_model,
            commands::device_models::delete_device_model,
            commands::exports::export_racks_excel,
            commands::exports::export_devices_data_excel,
            commands::exports::export_single_rack_excel,
            commands::exports::export_report_html,
            commands::exports::import_excel_from_path,
            commands::settings::get_logging_config,
            commands::settings::set_logging_enabled,
            commands::settings::open_log_dir,
        ])
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            log::error!("RackViz 启动失败: {}", e);
            std::process::exit(1);
        });
}
