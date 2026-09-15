use askama::Template;
use rusqlite::Connection;
use std::f64::consts::PI;
use crate::{db, error::AppError};
use crate::excel::device_type_label;
use crate::models::normalize_device_type;

/// 设备类型展示顺序（与前端 `constants/labels.ts` 的 `DEVICE_TYPES` 同序，§8-14）。
const TYPE_ORDER: [&str; 8] = [
    "server", "switch", "router", "storage", "nas", "security", "loadbalancer", "other",
];
/// 类型饼图配色（8 类固定色，与图例一致）。
const TYPE_COLORS: [&str; 8] = [
    "#1890ff", "#52c41a", "#faad14", "#722ed1", "#13c2c2", "#eb2f96", "#fa541c", "#8c8c8c",
];

// ===== 柱状图几何（与 templates/report.html 的网格线硬编码保持一致）=====
/// X 轴起点
const X0: f64 = 64.0;
/// X 轴终点
const X1: f64 = 640.0;
/// Y 轴底部（0%）
const Y_BOTTOM: f64 = 240.0;
/// 绘图区高度（100% 对应高度）
const PLOT_H: f64 = 200.0;
/// 柱子最大宽度
const BAR_MAX_W: f64 = 70.0;

#[derive(Template)]
#[template(path = "report.html")]
struct ReportTemplate {
    headers: Vec<String>,
    devices: Vec<ReportDevice>,
    total_devices: usize,
    online_count: usize,
    offline_count: usize,
    unconfigured_count: usize,
    // ===== N-17 图表（Rust 侧手写内联 SVG，零图表依赖 / 零 CDN）=====
    room_bars: Vec<RoomBar>,
    has_rooms: bool,
    type_slices: Vec<TypeSlice>,
    type_legend: Vec<LegendItem>,
    type_total: usize,
    has_types: bool,
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

/// 机房利用率柱状图的一根柱子。
struct RoomBar {
    label: String,
    used_u: i32,
    total_u: i32,
    percent_text: String,
    cx: f64,
    bar_x: f64,
    bar_y: f64,
    bar_w: f64,
    bar_h: f64,
    percent_y: f64,
    color: String,
}

/// 设备类型分布饼图的一段。
struct TypeSlice {
    label: String,
    color: String,
    count: usize,
    percent_text: String,
    path: String,
}

/// 饼图图例项（含 0 值类，保证图例稳定可用）。
struct LegendItem {
    label: String,
    color: String,
    count: usize,
    percent_text: String,
}

/// 机房使用率配色：>85% 红、≥60% 橙、其余绿。
fn util_color(pct: f64) -> &'static str {
    if pct > 85.0 {
        "#ff4d4f"
    } else if pct >= 60.0 {
        "#faad14"
    } else {
        "#52c41a"
    }
}

/// 截断过长标签（按字符数，超出尾部加省略号），避免柱状图 X 轴标签互相重叠。
fn truncate_label(s: &str, max_chars: usize) -> String {
    let mut it = s.chars();
    let head: String = it.by_ref().take(max_chars).collect();
    if it.next().is_some() {
        format!("{}…", head)
    } else {
        head
    }
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
        // 类型分布累计（无型号 / 未知类型 → other 兜底）
        let ty = d.device_model_id
            .and_then(|id| model_types.get(&id))
            .map(|s| normalize_device_type(s))
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

    // ===== ① 机房利用率（柱状图）=====
    // 每机柜已用 U 位（按设备 U 位区间求和；软删设备已被 list_devices 排除）
    let mut used_by_rack: std::collections::HashMap<i32, i32> = std::collections::HashMap::new();
    for d in &devices {
        if let (Some(rid), Some(s), Some(e)) = (d.rack_id, d.start_u, d.end_u) {
            if e >= s {
                *used_by_rack.entry(rid).or_insert(0) += e - s + 1;
            }
        }
    }

    // 每机房：总 U 位 = 该机房所有机柜高度之和；已用 U 位 = 这些机柜内设备占用之和
    let mut room_rows: Vec<(String, i32, i32)> = Vec::new();
    for room in &rooms {
        let room_racks: Vec<&crate::models::Rack> =
            racks.iter().filter(|r| r.room_id == Some(room.id)).collect();
        let total_u: i32 = room_racks.iter().map(|r| r.height_u.max(0)).sum();
        if total_u <= 0 {
            continue; // 无机柜的机房不参与利用率统计（避免除零）
        }
        let used_u: i32 = room_racks
            .iter()
            .map(|r| used_by_rack.get(&r.id).copied().unwrap_or(0))
            .sum();
        room_rows.push((room.name.clone(), used_u, total_u));
    }

    let n = room_rows.len();
    let slot = if n > 0 { (X1 - X0) / n as f64 } else { 0.0 };
    let bar_w = (slot * 0.6).clamp(10.0, BAR_MAX_W);
    let room_bars: Vec<RoomBar> = room_rows
        .iter()
        .enumerate()
        .map(|(i, (name, used_u, total_u))| {
            let pct = if *total_u > 0 {
                (*used_u as f64 / *total_u as f64) * 100.0
            } else {
                0.0
            };
            let bar_h = PLOT_H * (pct / 100.0);
            let cx = X0 + slot * i as f64 + slot / 2.0;
            RoomBar {
                label: truncate_label(name, 8),
                used_u: *used_u,
                total_u: *total_u,
                percent_text: format!("{:.0}%", pct),
                cx,
                bar_x: cx - bar_w / 2.0,
                bar_y: Y_BOTTOM - bar_h,
                bar_w,
                bar_h,
                percent_y: (Y_BOTTOM - bar_h) - 5.0,
                color: util_color(pct).to_string(),
            }
        })
        .collect();

    // ===== ② 设备类型分布（饼图）=====
    let type_total: usize = type_counts.iter().sum();
    let mut type_slices: Vec<TypeSlice> = Vec::new();
    let mut type_legend: Vec<LegendItem> = Vec::new();

    for i in 0..8 {
        let count = type_counts[i];
        let pct = if type_total > 0 {
            (count as f64 / type_total as f64) * 100.0
        } else {
            0.0
        };
        type_legend.push(LegendItem {
            label: device_type_label(TYPE_ORDER[i]).to_string(),
            color: TYPE_COLORS[i].to_string(),
            count,
            percent_text: format!("{:.1}%", pct),
        });
    }

    if type_total > 0 {
        let cx = 150.0f64;
        let cy = 150.0f64;
        let r = 120.0f64;
        let nonzero = type_counts.iter().filter(|c| **c > 0).count();
        let single = nonzero == 1;
        let mut start = -PI / 2.0; // 从正上方开始
        for i in 0..8 {
            let count = type_counts[i];
            if count == 0 {
                continue;
            }
            let frac = count as f64 / type_total as f64;
            let end = start + frac * 2.0 * PI;
            let path = if single {
                // 整圆：两段半圆弧
                format!(
                    "M {cx} {cy} L {cx} {top} A {r} {r} 0 1 1 {cx} {bot} A {r} {r} 0 1 1 {cx} {top} Z",
                    top = cy - r,
                    bot = cy + r
                )
            } else {
                let x0 = cx + r * start.cos();
                let y0 = cy + r * start.sin();
                let x1 = cx + r * end.cos();
                let y1 = cy + r * end.sin();
                let large = if (end - start) > PI { 1 } else { 0 };
                format!(
                    "M {cx:.2} {cy:.2} L {x0:.2} {y0:.2} A {r:.2} {r:.2} 0 {large} 1 {x1:.2} {y1:.2} Z"
                )
            };
            type_slices.push(TypeSlice {
                label: device_type_label(TYPE_ORDER[i]).to_string(),
                color: TYPE_COLORS[i].to_string(),
                count,
                percent_text: format!("{:.1}%", frac * 100.0),
                path,
            });
            start = end;
        }
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
        has_rooms: !room_bars.is_empty(),
        room_bars,
        has_types: type_total > 0,
        type_slices,
        type_legend,
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
        // 内联 SVG：不得引入任何外部资源
        assert!(html.contains("<svg"), "报表应含内联 SVG");
        assert!(!html.contains("<script"), "报表不得含脚本");
    }

    #[test]
    fn test_report_pie_no_data_is_graceful() {
        // 空库：不 panic、饼图区域显示占位文案
        let conn = setup_db();
        let html = render_report(&conn).unwrap();
        assert!(html.contains("暂无设备数据"));
    }
}
