use askama::Template;
use rusqlite::Connection;
use crate::{db, error::AppError};

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate {
    headers: Vec<String>,
    devices: Vec<ReportDevice>,
    total_devices: usize,
    online_count: usize,
    offline_count: usize,
    unconfigured_count: usize,
}

struct ReportDevice {
    name: String,
    model: String,
    room: String,
    rack: String,
    ip: String,
    status_label: String,
    status_class: String,
    department: String,
    owner: String,
}

pub fn render_report(conn: &Connection) -> Result<String, AppError> {
    let devices = db::devices::list_devices(conn, None, None)?;
    let models_map = db::device_models::list_models_map(conn)?;
    let racks_map = db::racks::list_racks_map(conn)?;
    let rooms_map = db::racks::list_rooms_map(conn)?;

    let mut online = 0usize;
    let mut offline = 0usize;
    let mut unconfigured = 0usize;

    let report_devices: Vec<ReportDevice> = devices.iter().map(|d| {
        match d.status.as_str() {
            "online" => online += 1,
            "offline" => offline += 1,
            _ => unconfigured += 1,
        }
        let (status_label, status_class) = match d.status.as_str() {
            "online" => ("开机".into(), "online".into()),
            "offline" => ("离线".into(), "offline".into()),
            _ => ("未上架".into(), "unconfigured".into()),
        };

        // 解析关联数据
        let model_name = d.device_model_id
            .and_then(|id| models_map.get(&id))
            .cloned()
            .unwrap_or_default();
        let rack_info = d.rack_id.and_then(|id| racks_map.get(&id));
        let rack_name = rack_info.map(|(n, _)| n.as_str()).unwrap_or("").to_string();
        let room_name = rack_info
            .and_then(|(_, room_id)| *room_id)
            .and_then(|rid| rooms_map.get(&rid))
            .cloned()
            .unwrap_or_default();

        ReportDevice {
            name: d.name.clone(),
            model: model_name,
            room: room_name,
            rack: rack_name,
            ip: d.ip_addresses.clone(),
            status_label,
            status_class,
            department: d.department.clone(),
            owner: d.owner.clone(),
        }
    }).collect();

    let template = ReportTemplate {
        headers: vec![
            "设备名称".into(), "型号".into(), "机房".into(), "机柜".into(),
            "IP地址".into(), "状态".into(), "部门".into(), "责任人".into(),
        ],
        devices: report_devices,
        total_devices: devices.len(),
        online_count: online,
        offline_count: offline,
        unconfigured_count: unconfigured,
    };

    template.render().map_err(|e| AppError::io(&format!("报表生成失败: {}", e)))
}
