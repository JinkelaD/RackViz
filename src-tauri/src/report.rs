use askama::Template;
use rusqlite::Connection;
use crate::{db, error::AppError};

/// 设备类型展示顺序 / 标签 / 配色（与前端 `constants/labels.ts` 的 `DEVICE_TYPES` 同序，§8-14）。
const TYPE_ORDER: [&str; 8] = [
    "server", "switch", "router", "storage", "nas", "security", "loadbalancer", "other",
];
const TYPE_LABELS: [&str; 8] = [
    "服务器", "交换机", "路由器", "存储阵列", "NAS 存储", "网安设备", "负载均衡", "其他",
];
const TYPE_COLORS: [&str; 8] = [
    "#2f80ed", "#00b8d9", "#f2a900", "#9b51e0", "#27ae60", "#e0492a", "#e0489a", "#8c8c8c",
];

/// 柱状图基准宽度（机房使用率）
const BAR_MAX_PX: i32 = 320;
/// 堆叠条基准宽度（类型分布）
const STACK_MAX_PX: i32 = 480;

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate {
    headers: Vec<String>,
    devices: Vec<ReportDevice>,
    total_devices: usize,
    online_count: usize,
    offline_count: usize,
    unconfigured_count: usize,
    // ===== N-17 图表（手写内联 SVG，零图表依赖）=====
    room_utils: Vec<RoomUtil>,
    type_slices: Vec<TypeSlice>,
    type_total: usize,
    has_rooms: bool,
    has_types: bool,
    bar_chart_height: i32,
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

/// 机房 U 位使用率（柱状图一行）
struct RoomUtil {
    name: String,
    used_u: i32,
    total_u: i32,
    pct: i32,
    bar_px: i32,
    y: i32,
    text_y: i32,
}

/// 设备类型分布（堆叠条一段）
struct TypeSlice {
    label: String,
    color: String,
    count: usize,
    pct: i32,
    x: i32,
    width: i32,
}

pub fn render_report(conn: &Connection) -> Result<String, AppError> {
    let devices = db::devices::list_devices(conn, None, None)?;
    let rooms = db::rooms::list_rooms(conn)?;
    let racks = db::racks::list_racks(conn)?;
    let models_map = db::device_models::list_models_map(conn)?;
    let racks_map = db::racks::list_racks_map(conn)?;
    let rooms_map = db::racks::list_rooms_map(conn)?;
    let model_types: std::collections::HashMap<i32, String> = db::device_models::list_device_models(conn)?
        .into_iter()
        .map(|m| (m.id, m.device_type))
        .collect();

    let mut online = 0usize;
    let mut offline = 0usize;
    let mut unconfigured = 0usize;
    let mut type_counts = [0usize; 8];

    let report_devices: Vec<ReportDevice> = devices.iter().map(|d| {
        match d.status.as_str() {
            "online" => online += 1,
            "offline" => offline += 1,
            _ => unconfigured += 1,
        }
        // 类型分布累计（无型号 → other）
        let ty = d.device_model_id
            .and_then(|id| model_types.get(&id))
            .map(|s| s.as_str())
            .unwrap_or("other");
        let ti = TYPE_ORDER.iter().position(|t| *t == ty).unwrap_or(TYPE_ORDER.len() - 1);
        type_counts[ti] += 1;

        let (status_label, status_class) = match d.status.as_str() {
            "online" => ("开机".into(), "online".into()),
            "offline" => ("离线".into(), "offline".into()),
            _ => ("未上架".into(), "unconfigured".into()),
        };

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

    // ===== 机房 U 位使用率（柱状）=====
    // 每台设备的有效占用高度（height_u ≥ 1）
    let mut room_utils: Vec<RoomUtil> = Vec::new();
    for (idx, room) in rooms.iter().enumerate() {
        let rack_ids: std::collections::HashSet<i32> = racks.iter()
            .filter(|r| r.room_id == Some(room.id))
            .map(|r| r.id)
            .collect();
        let total_u: i32 = racks.iter()
            .filter(|r| r.room_id == Some(room.id))
            .map(|r| r.height_u.max(0))
            .sum();
        let used_u: i32 = if rack_ids.is_empty() {
            0
        } else {
            devices.iter()
                .filter(|d| d.rack_id.map(|rid| rack_ids.contains(&rid)).unwrap_or(false))
                .map(|d| d.height_u.max(0))
                .sum()
        };
        let pct = if total_u > 0 {
            ((used_u * 100) / total_u).clamp(0, 100)
        } else {
            0
        };
        let bar_px = BAR_MAX_PX * pct / 100;
        let y = idx as i32 * 34 + 12;
        room_utils.push(RoomUtil {
            name: room.name.clone(),
            used_u,
            total_u,
            pct,
            bar_px,
            y,
            text_y: y + 12,
        });
    }
    let bar_chart_height = room_utils.len() as i32 * 34 + 16;

    // ===== 设备类型分布（堆叠条）=====
    let type_total: usize = type_counts.iter().sum();
    let mut type_slices: Vec<TypeSlice> = Vec::new();
    let mut cursor_x = 0i32;
    for i in 0..8 {
        let count = type_counts[i];
        if count == 0 {
            continue;
        }
        let width = if type_total > 0 {
            (STACK_MAX_PX as usize * count / type_total) as i32
        } else {
            0
        };
        let pct = if type_total > 0 {
            (count * 100 / type_total) as i32
        } else {
            0
        };
        type_slices.push(TypeSlice {
            label: TYPE_LABELS[i].to_string(),
            color: TYPE_COLORS[i].to_string(),
            count,
            pct,
            x: cursor_x,
            width,
        });
        cursor_x += width;
    }

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
        has_rooms: !room_utils.is_empty(),
        has_types: type_total > 0,
        bar_chart_height,
        room_utils,
        type_slices,
        type_total,
    };

    template.render().map_err(|e| AppError::io(&format!("报表生成失败: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::migration;
    use crate::models::DeviceCreate;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        migration::run(&conn).unwrap();
        conn
    }

    #[test]
    fn test_report_excludes_soft_deleted() {
        let conn = setup_db();
        db::devices::insert_device(&conn, &DeviceCreate { name: "KEEP".into(), ..Default::default() }).unwrap();
        let gone = db::devices::insert_device(&conn, &DeviceCreate { name: "GONE".into(), ..Default::default() }).unwrap();
        db::devices::soft_delete_device(&conn, gone.id).unwrap();

        let html = render_report(&conn).unwrap();
        assert!(html.contains("KEEP"), "在用设备应出现在报表");
        assert!(!html.contains("GONE"), "软删设备不得出现在报表");
    }

    #[test]
    fn test_report_computes_room_utilization_and_types() {
        let conn = setup_db();
        db::rooms::insert_room(&conn, &crate::models::RoomCreate { name: "A机房".into(), ..Default::default() }).unwrap();
        db::racks::insert_rack(&conn, &crate::models::RackCreate {
            name: "R01".into(), room_id: Some(1), height_u: Some(10), ..Default::default()
        }).unwrap();
        // 一台 4U 设备（交换机型号）挂在 R01
        let mid = db::device_models::find_or_create_model(&conn, "H3C-SW", Some("switch")).unwrap();
        db::devices::insert_device(&conn, &DeviceCreate {
            name: "SW1".into(),
            device_model_id: Some(mid),
            rack_id: Some(1),
            start_u: Some(1),
            end_u: Some(4),
            ..Default::default()
        }).unwrap();

        let html = render_report(&conn).unwrap();
        assert!(html.contains("A机房"));
        // 使用率 4/10 = 40%
        assert!(html.contains("40%"), "使用率应为 40%");
        assert!(html.contains("交换机"), "类型分布应含交换机");
    }
}
