use rusqlite::Connection;
use rust_xlsxwriter::*;
use crate::{db, error::AppError, models::*};

fn format_position(start_u: Option<i32>, end_u: Option<i32>) -> String {
    match (start_u, end_u) {
        (Some(s), Some(e)) => format!("{}-{}U", s, e),
        (Some(u), None) => format!("{}U", u),
        _ => String::new(),
    }
}

fn format_date(d: Option<chrono::NaiveDate>) -> String {
    d.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default()
}

fn get_status_label(status: &str) -> &str {
    match status {
        "online" => "开机",
        "offline" => "离线",
        "unconfigured" => "未上架",
        _ => status,
    }
}

/// 解析多种日期格式
#[allow(dead_code)]
fn parse_flexible_date(s: &str) -> Option<chrono::NaiveDate> {
    let s = s.trim();
    if s.is_empty() { return None; }
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y/%m/%d"))
        .or_else(|_| chrono::NaiveDate::parse_from_str(s, "%Y.%m.%d"))
        .ok()
}

/// 解析 U 位范围字符串
fn parse_u_range(s: &str) -> Option<(i32, i32)> {
    let s = s.trim().trim_end_matches('U').trim_end_matches('u');
    if let Some(dash_pos) = s.find('-') {
        let start: i32 = s[..dash_pos].trim().parse().ok()?;
        let end: i32 = s[dash_pos+1..].trim().parse().ok()?;
        Some((start, end))
    } else {
        let u: i32 = s.parse().ok()?;
        Some((u, u))
    }
}

/// 从 calamine::Data 提取字符串值
fn cell_to_string(cell: &calamine::Data) -> String {
    match cell {
        calamine::Data::String(s) => s.clone(),
        calamine::Data::Float(f) => f.to_string(),
        calamine::Data::Int(i) => i.to_string(),
        calamine::Data::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// 从 calamine::Data 提取整数
fn cell_to_int(cell: &calamine::Data) -> Option<i32> {
    match cell {
        calamine::Data::Float(f) => Some(*f as i32),
        calamine::Data::Int(i) => Some(*i as i32),
        calamine::Data::String(s) => s.parse().ok(),
        _ => None,
    }
}

pub fn export_devices_data_excel(conn: &Connection) -> Result<Vec<u8>, AppError> {
    let devices = db::devices::list_devices(conn, None, None)?;
    let models_map = db::device_models::list_models_map(conn)?;
    let racks_map = db::racks::list_racks_map(conn)?;
    let rooms_map = db::racks::list_rooms_map(conn)?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("设备台账")?;

    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE8EEF4))
        .set_border(FormatBorder::Thin);

    let headers: [&str; 15] = [
        "设备名称", "型号", "机房", "机柜", "位置(U)",
        "IP地址", "序列号", "资产编号", "使用部门", "责任人",
        "功能描述", "采购日期", "质保到期", "状态", "功率(W)",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (row_idx, dev) in devices.iter().enumerate() {
        let r = (row_idx + 1) as u32;

        let model_name = dev.device_model_id
            .and_then(|id| models_map.get(&id))
            .map(|n| n.as_str())
            .unwrap_or("");
        let rack_info = dev.rack_id.and_then(|id| racks_map.get(&id));
        let rack_name = rack_info.map(|(n, _)| n.as_str()).unwrap_or("");
        let room_name = rack_info
            .and_then(|(_, room_id)| *room_id)
            .and_then(|rid| rooms_map.get(&rid))
            .map(|n| n.as_str())
            .unwrap_or("");

        sheet.write_string(r, 0, &dev.name)?;
        sheet.write_string(r, 1, model_name)?;
        sheet.write_string(r, 2, room_name)?;
        sheet.write_string(r, 3, rack_name)?;
        sheet.write_string(r, 4, format_position(dev.start_u, dev.end_u))?;
        sheet.write_string(r, 5, &dev.ip_addresses)?;
        sheet.write_string(r, 6, &dev.serial_no)?;
        sheet.write_string(r, 7, &dev.asset_no)?;
        sheet.write_string(r, 8, &dev.department)?;
        sheet.write_string(r, 9, &dev.owner)?;
        sheet.write_string(r, 10, &dev.function)?;
        sheet.write_string(r, 11, format_date(dev.purchase_date))?;
        sheet.write_string(r, 12, format_date(dev.warranty_expire))?;
        sheet.write_string(r, 13, get_status_label(&dev.status))?;
        sheet.write_number(r, 14, dev.power_watt as f64)?;
    }

    for col in 0..15u16 {
        sheet.set_column_width(col, 14.0)?;
    }

    Ok(workbook.save_to_buffer()?)
}

pub fn export_racks_excel(conn: &Connection) -> Result<Vec<u8>, AppError> {
    let racks = db::racks::list_racks(conn)?;
    // 避免 N+1：一次取出全部设备，按 rack_id 内存分组
    let devices = db::devices::list_devices(conn, None, None)?;
    let mut by_rack: std::collections::HashMap<Option<i32>, Vec<&Device>> =
        std::collections::HashMap::new();
    for d in &devices {
        by_rack.entry(d.rack_id).or_default().push(d);
    }

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("机柜部署图")?;

    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE8EEF4));

    let max_u = racks.iter().map(|r| r.height_u).max().unwrap_or(42);
    sheet.write_string(0, 0, "U位")?;

    for (rack_idx, rack) in racks.iter().enumerate() {
        sheet.write_string_with_format(0, (rack_idx + 1) as u16, &rack.name, &header_fmt)?;
    }

    for u in (1..=max_u).rev() {
        let row = (max_u - u + 1) as u32;
        sheet.write_string(row, 0, format!("{}U", u))?;
    }

    for (idx, rack) in racks.iter().enumerate() {
        let rack_col = (idx + 1) as u16;
        let rack_devices = by_rack.get(&Some(rack.id)).into_iter().flatten().copied();
        for dev in rack_devices {
            if let (Some(start), Some(end)) = (dev.start_u, dev.end_u) {
                for u in start..=end {
                    if u >= 1 && u <= rack.height_u {
                        let row = (rack.height_u - u + 1) as u32;
                        sheet.write_string(row, rack_col, &dev.name)?;
                    }
                }
            }
        }
    }

    Ok(workbook.save_to_buffer()?)
}

pub fn export_single_rack_excel(conn: &Connection, rack_id: i32) -> Result<Vec<u8>, AppError> {
    let rack = db::racks::get_rack(conn, rack_id)?
        .ok_or_else(|| AppError::not_found("机柜"))?;
    let devices = db::devices::list_devices(conn, Some(rack_id), None)?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name(&rack.name)?;

    let header_fmt = Format::new().set_bold().set_background_color(Color::RGB(0xE8EEF4));
    sheet.write_string_with_format(0, 0, "U位", &header_fmt)?;
    sheet.write_string_with_format(0, 1, &rack.name, &header_fmt)?;

    for u in (1..=rack.height_u).rev() {
        let row = (rack.height_u - u + 1) as u32;
        sheet.write_string(row, 0, format!("{}U", u))?;
    }

    for dev in &devices {
        if let (Some(start), Some(end)) = (dev.start_u, dev.end_u) {
            for u in start..=end {
                if u >= 1 && u <= rack.height_u {
                    let row = (rack.height_u - u + 1) as u32;
                    sheet.write_string(row, 1, &dev.name)?;
                }
            }
        }
    }

    sheet.set_column_width(0, 8.0)?;
    sheet.set_column_width(1, 20.0)?;
    Ok(workbook.save_to_buffer()?)
}

/// 从 Excel 文件导入设备（含重复检测）
pub fn import_devices_excel(conn: &Connection, file_path: &str) -> Result<ImportResult, AppError> {
    use calamine::{open_workbook, Reader, Xlsx};

    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .map_err(|e| AppError::io(&format!("无法打开Excel文件: {}", e)))?;

    let range = workbook.worksheet_range_at(0)
        .ok_or_else(|| AppError::validation("Excel文件中未找到工作表"))?
        .map_err(|e| AppError::io(&format!("读取工作表失败: {}", e)))?;

    let mut imported = 0u32;
    let mut skipped = 0u32;
    let mut errors = Vec::new();
    let max_rows = 5000u32;

    db::with_transaction(conn, |c| {
        for (row_idx, row) in range.rows().enumerate() {
            if row_idx == 0 { continue; }
            if imported + skipped >= max_rows {
                errors.push(format!("超过5000行上限，第{}行及之后被跳过", row_idx + 1));
                break;
            }

            let name = cell_to_string(row.first().unwrap_or(&calamine::Data::Empty)).trim().to_string();
            if name.is_empty() { continue; }

            let model_name = cell_to_string(row.get(1).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let model_id = if !model_name.is_empty() {
                Some(db::device_models::find_or_create_model(c, &model_name)?)
            } else {
                None
            };

            let _room_name = cell_to_string(row.get(2).unwrap_or(&calamine::Data::Empty)).trim().to_string();

            let rack_name = cell_to_string(row.get(3).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let rack_id = if !rack_name.is_empty() {
                Some(db::racks::find_or_create_rack(c, &rack_name)?)
            } else {
                None
            };

            let u_str = cell_to_string(row.get(4).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let (start_u, end_u) = parse_u_range(&u_str).unzip();

            let ip_addresses = cell_to_string(row.get(5).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let serial_no = cell_to_string(row.get(6).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let asset_no = cell_to_string(row.get(7).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let department = cell_to_string(row.get(8).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let owner = cell_to_string(row.get(9).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let function_desc = cell_to_string(row.get(10).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let purchase_date_str = cell_to_string(row.get(11).unwrap_or(&calamine::Data::Empty)).trim().to_string();
            let warranty_expire_str = cell_to_string(row.get(12).unwrap_or(&calamine::Data::Empty)).trim().to_string();

            let status_raw = cell_to_string(row.get(13).unwrap_or(&calamine::Data::Empty));
            let status = match status_raw.trim() {
                "开机" | "online" => "online".to_string(),
                "离线" | "offline" => "offline".to_string(),
                _ => "unconfigured".to_string(),
            };

            let power_watt = cell_to_int(row.get(14).unwrap_or(&calamine::Data::Empty)).unwrap_or(0);

            // ===== 重复检测 =====
            // 1. 序列号唯一性检查
            if !serial_no.is_empty() {
                if let Some(existing) = db::devices::find_device_by_serial(c, &serial_no)? {
                    log::warn!("导入跳过第{}行: 序列号重复（已存在设备 id={}）", row_idx + 1, existing.id);
                    skipped += 1;
                    continue;
                }
            }

            // 2. 资产编号唯一性检查
            if !asset_no.is_empty() {
                if let Some(existing) = db::devices::find_device_by_asset(c, &asset_no)? {
                    log::warn!("导入跳过第{}行: 资产编号重复（已存在设备 id={}）", row_idx + 1, existing.id);
                    skipped += 1;
                    continue;
                }
            }

            // 3. 同机柜内名称+位置冲突检查
            if let Some(existing) = db::devices::find_device_by_name_in_rack(c, &name, rack_id)? {
                log::warn!("导入跳过第{}行: 同机柜设备名称冲突（已存在设备 id={}）", row_idx + 1, existing.id);
                skipped += 1;
                continue;
            }

            let data = DeviceCreate {
                name,
                device_model_id: model_id,
                rack_id,
                start_u,
                end_u,
                ip_addresses: if ip_addresses.is_empty() { None } else { Some(ip_addresses) },
                serial_no: if serial_no.is_empty() { None } else { Some(serial_no) },
                asset_no: if asset_no.is_empty() { None } else { Some(asset_no) },
                department: if department.is_empty() { None } else { Some(department) },
                owner: if owner.is_empty() { None } else { Some(owner) },
                function: if function_desc.is_empty() { None } else { Some(function_desc) },
                purchase_date: if purchase_date_str.is_empty() { None } else { Some(purchase_date_str) },
                warranty_expire: if warranty_expire_str.is_empty() { None } else { Some(warranty_expire_str) },
                status: Some(status),
                power_watt: if power_watt > 0 { Some(power_watt) } else { None },
                // 高度由 insert_device 推导：U 位区间优先，其次型号高度（None 不显式指定）
                height_u: None,
            };

            db::devices::insert_device(c, &data)?;
            imported += 1;
        }
        Ok(())
    })?;

    Ok(ImportResult { imported, skipped, errors })
}
