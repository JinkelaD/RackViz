use rusqlite::Connection;
use rust_xlsxwriter::*;
use crate::{db, error::AppError, models::*};

/// 设备类型英文枚举 → 中文标签（N-21）。
///
/// 与前端 `constants/labels.ts::DEVICE_TYPE_LABELS` **双源同步**（§8-14）；
/// 任一侧新增/改名必须同步，否则导出往返会落到 `其他`。
pub fn device_type_label(ty: &str) -> &'static str {
    match ty {
        "server" => "服务器",
        "switch" => "交换机",
        "router" => "路由器",
        "storage" => "存储阵列",
        "nas" => "NAS 存储",
        "security" => "网安设备",
        "loadbalancer" => "负载均衡",
        "other" => "其他",
        _ => "其他",
    }
}

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

/// 空串 → `None`（写入 DB 时统一转 NULL / 空语义）
fn none_if_empty(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

/// 文本字段的三态：空串 → `Unset`（保留原值），非空 → `Set`。
fn patch_str(s: &str) -> Patch<String> {
    if s.is_empty() { Patch::Unset } else { Patch::Set(s.to_string()) }
}

/// 归一化键：trim → 小写 → 去空白/`_`/`-`（与 `models::normalize_device_type` 内部规则一致）。
fn normalize_key(raw: &str) -> String {
    raw.trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect()
}

/// 判断原始值是否本身就是已知的「其他」写法（用于区分兜底 `other` 与显式 `其他`）。
fn is_known_other(raw: &str) -> bool {
    let k = normalize_key(raw);
    k == "other" || k == "其他"
}

/// 解析/新建型号（N-21）。
///
/// - 名称空 → `Ok(None)`（不创建型号）；
/// - 同名已存在 → 复用其 id、**不覆盖既有 type**；仅当传入类型与既有类型（归一化后）不同
///   才追加告警 `型号「X」已存在，类型保持为 Y`（Y 为既有 type 的中文标签）；
/// - 不存在 → 新建，累加 `models_created`。
fn resolve_model(
    conn: &Connection,
    name: &str,
    device_type: Option<&str>,
    models_created: &mut u32,
    warnings: &mut Vec<String>,
) -> Result<Option<i32>, AppError> {
    if name.is_empty() {
        return Ok(None);
    }
    if let Some(existing) = db::device_models::get_device_model_by_name(conn, name)? {
        if let Some(dt) = device_type {
            if normalize_device_type(&existing.device_type) != dt {
                warnings.push(format!(
                    "型号「{}」已存在，类型保持为 {}",
                    name,
                    device_type_label(normalize_device_type(&existing.device_type))
                ));
            }
        }
        Ok(Some(existing.id))
    } else {
        let id = db::device_models::find_or_create_model(conn, name, device_type)?;
        *models_created += 1;
        Ok(Some(id))
    }
}

/// 关联机房（N-05）：`link_room=true` 且「机房」列非空且机柜存在时，
/// 幂等 `find_or_create_room` 取 id，并把机柜的 `room_id` 写回该机房
/// （机房不直接挂 devices，而是经 `devices.rack_id → racks.room_id → rooms` 链路）。
///
/// 复用 `db::racks::link_rack_room`：仅当机柜当前无归属（`room_id IS NULL`）时写入，
/// 不覆盖用户在机柜管理中的手工归属（幂等、非破坏）。
fn link_rack_room(
    conn: &Connection,
    rack_id: Option<i32>,
    room_name: &str,
    link: bool,
) -> Result<(), AppError> {
    if !link {
        return Ok(());
    }
    let Some(rid) = rack_id else { return Ok(()); };
    let room_name = room_name.trim();
    if room_name.is_empty() {
        return Ok(());
    }
    let room_id = db::rooms::find_or_create_room(conn, room_name)?;
    db::racks::link_rack_room(conn, rid, room_id)
}

/// 构造覆盖更新（`update_mode = "overwrite"`）的三态 Patch：
/// 仅"文件提供了值"的字段才写（空串→`Unset`，保留原值），避免误清空用户数据。
#[allow(clippy::too_many_arguments)]
fn build_overwrite_update(
    name: &str,
    model_id: Option<i32>,
    rack_id: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
    ip: &str,
    serial: &str,
    asset: &str,
    department: &str,
    owner: &str,
    function_desc: &str,
    purchase: &str,
    warranty: &str,
    status: &str,
    power_watt: i32,
) -> DeviceUpdate {
    DeviceUpdate {
        name: Patch::Set(name.to_string()),
        device_model_id: model_id.map(Patch::Set).unwrap_or(Patch::Unset),
        rack_id: rack_id.map(Patch::Set).unwrap_or(Patch::Unset),
        start_u: start_u.map(Patch::Set).unwrap_or(Patch::Unset),
        end_u: end_u.map(Patch::Set).unwrap_or(Patch::Unset),
        ip_addresses: patch_str(ip),
        serial_no: patch_str(serial),
        asset_no: patch_str(asset),
        department: patch_str(department),
        owner: patch_str(owner),
        function: patch_str(function_desc),
        purchase_date: patch_str(purchase),
        warranty_expire: patch_str(warranty),
        status: Patch::Set(status.to_string()),
        power_watt: if power_watt > 0 { Patch::Set(power_watt) } else { Patch::Unset },
        height_u: Patch::Unset,
    }
}

/// 导出设备台账（Q7 增强 + N-21 第 16 列）。
///
/// - 冻结首行 + 自动筛选覆盖全 16 列；
/// - `功率(W)` 列（索引 14）加数据条（DataBar）；
/// - 第 16 列「设备类型」写中文标签，保证 export → import 往返后 `device_models.type` 不丢失。
pub fn export_devices_data_excel(conn: &Connection) -> Result<Vec<u8>, AppError> {
    let devices = db::devices::list_devices(conn, None, None)?;
    let model_list = db::device_models::list_device_models(conn)?;
    let model_names: std::collections::HashMap<i32, String> =
        model_list.iter().map(|m| (m.id, m.name.clone())).collect();
    let model_types: std::collections::HashMap<i32, String> =
        model_list.iter().map(|m| (m.id, m.device_type.clone())).collect();
    let racks_map = db::racks::list_racks_map(conn)?;
    let rooms_map = db::racks::list_rooms_map(conn)?;

    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet().set_name("设备台账")?;

    let header_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE8EEF4))
        .set_border(FormatBorder::Thin);

    let headers: [&str; 16] = [
        "设备名称", "型号", "机房", "机柜", "位置(U)",
        "IP地址", "序列号", "资产编号", "使用部门", "责任人",
        "功能描述", "采购日期", "质保到期", "状态", "功率(W)",
        "设备类型",
    ];

    for (col, h) in headers.iter().enumerate() {
        sheet.write_string_with_format(0, col as u16, *h, &header_fmt)?;
    }

    for (row_idx, dev) in devices.iter().enumerate() {
        let r = (row_idx + 1) as u32;

        let model_name = dev.device_model_id
            .and_then(|id| model_names.get(&id))
            .map(|n| n.as_str())
            .unwrap_or("");
        let rack_info = dev.rack_id.and_then(|id| racks_map.get(&id));
        let rack_name = rack_info.map(|(n, _)| n.as_str()).unwrap_or("");
        let room_name = rack_info
            .and_then(|(_, room_id)| *room_id)
            .and_then(|rid| rooms_map.get(&rid))
            .map(|n| n.as_str())
            .unwrap_or("");
        let type_label = dev.device_model_id
            .and_then(|id| model_types.get(&id))
            .map(|t| device_type_label(normalize_device_type(t)))
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
        sheet.write_string(r, 15, type_label)?;
    }

    // 冻结首行 + 自动筛选覆盖全 16 列
    let last_row = devices.len() as u32;
    sheet.set_freeze_panes(1, 0)?;
    sheet.autofilter(0, 0, last_row, 15)?;

    // ① 设备台账：功率(W) 列（索引 14）加经典数据条
    if !devices.is_empty() {
        let data_bar = ConditionalFormatDataBar::new()
            .set_fill_color(Color::RGB(0x63BE7B))
            .use_classic_style();
        sheet.add_conditional_format(1, 14, last_row, 14, &data_bar)?;
    }

    for col in 0..16u16 {
        sheet.set_column_width(col, 14.0)?;
    }

    Ok(workbook.save_to_buffer()?)
}

/// 导出机柜部署图（Q7 增强）。
///
/// - 冻结首行 + 自动筛选覆盖全列；
/// - ② 第 1 行为「使用率」行，逐机柜写 U 位使用率（百分比数值）并加三色色阶（ColorScale）；
/// - U 位网格自第 2 行起。
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
    let usage_fmt = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xF0F5FA))
        .set_border(FormatBorder::Thin);

    let max_u = racks.iter().map(|r| r.height_u).max().unwrap_or(42);
    sheet.write_string_with_format(0, 0, "U位", &header_fmt)?;

    for (rack_idx, rack) in racks.iter().enumerate() {
        sheet.write_string_with_format(0, (rack_idx + 1) as u16, &rack.name, &header_fmt)?;
    }

    // 第 1 行：使用率（百分比数值，供色阶使用）
    sheet.write_string_with_format(1, 0, "使用率", &usage_fmt)?;
    for (idx, rack) in racks.iter().enumerate() {
        let used_u: i32 = by_rack
            .get(&Some(rack.id))
            .into_iter()
            .flatten()
            .filter_map(|d| match (d.start_u, d.end_u) {
                (Some(s), Some(e)) if e >= s => Some(e - s + 1),
                _ => None,
            })
            .sum();
        let pct = if rack.height_u > 0 {
            used_u as f64 / rack.height_u as f64 * 100.0
        } else {
            0.0
        };
        sheet.write_number_with_format(1, (idx + 1) as u16, pct, &usage_fmt)?;
    }

    // U 位网格（自第 2 行起）
    for u in (1..=max_u).rev() {
        let row = (max_u - u + 2) as u32;
        sheet.write_string(row, 0, format!("{}U", u))?;
    }

    for (idx, rack) in racks.iter().enumerate() {
        let rack_col = (idx + 1) as u16;
        let rack_devices = by_rack.get(&Some(rack.id)).into_iter().flatten().copied();
        for dev in rack_devices {
            if let (Some(start), Some(end)) = (dev.start_u, dev.end_u) {
                for u in start..=end {
                    if u >= 1 && u <= rack.height_u {
                        let row = (rack.height_u - u + 2) as u32;
                        sheet.write_string(row, rack_col, &dev.name)?;
                    }
                }
            }
        }
    }

    // 冻结首行 + 自动筛选覆盖全列
    let last_row = (max_u + 1) as u32;
    let last_col = racks.len() as u16;
    sheet.set_freeze_panes(1, 0)?;
    sheet.autofilter(0, 0, last_row, last_col)?;

    // ② 机柜部署图：使用率行加三色色阶
    if !racks.is_empty() {
        let color_scale = ConditionalFormat3ColorScale::new();
        sheet.add_conditional_format(1, 1, 1, last_col, &color_scale)?;
    }

    sheet.set_column_width(0, 8.0)?;
    for c in 1..=last_col {
        sheet.set_column_width(c, 16.0)?;
    }

    Ok(workbook.save_to_buffer()?)
}

/// 导出单机柜（Q7 增强：冻结首行 + 自动筛选覆盖全列 + 列宽）。
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

    let last_row = rack.height_u.max(0) as u32;
    sheet.set_freeze_panes(1, 0)?;
    sheet.autofilter(0, 0, last_row, 1)?;
    sheet.set_column_width(0, 8.0)?;
    sheet.set_column_width(1, 20.0)?;
    Ok(workbook.save_to_buffer()?)
}

/// 从 Excel 文件导入设备（T2.5 + T2.8）。
///
/// - `options.update_mode`：`"overwrite"` 命中已存在记录 → 更新并计入 `updated`；
///   其余（含 `"skip"`）→ 跳过并计入 `skipped`。
/// - `options.link_room`：为 true 且「机房」列非空时，幂等关联机房（写回机柜归属）。
/// - 重复检测：`find_device_by_serial` / `find_device_by_asset` / `find_device_by_name_in_rack`
///   （三者均已排除软删记录）。
/// - N-21：解析第 16 列（索引 15）「设备类型」，旧文件缺失 → `None`（新建型号按 `'server'` 兜底，100% 向后兼容）。
/// - 进度：通过 `on_progress(processed, total, phase)` 回调上报（由命令层转 `emit`）。
/// - `ImportResult.total` = 本次解析到的数据行数（不含表头、不含空名称行）。
pub fn import_devices_excel(
    conn: &Connection,
    file_path: &str,
    options: &ImportOptions,
    on_progress: &dyn Fn(u32, u32, &str),
) -> Result<ImportResult, AppError> {
    use calamine::{open_workbook, Reader, Xlsx};

    const MAX_ROWS: u32 = 5000;
    const PROGRESS_EVERY: u32 = 50;

    let update_mode = if options.update_mode == "overwrite" { "overwrite" } else { "skip" };

    let mut workbook: Xlsx<_> = open_workbook(file_path)
        .map_err(|e| AppError::io(&format!("无法打开Excel文件: {}", e)))?;

    let range = workbook.worksheet_range_at(0)
        .ok_or_else(|| AppError::validation("Excel文件中未找到工作表"))?
        .map_err(|e| AppError::io(&format!("读取工作表失败: {}", e)))?;

    // ===== 解析阶段 =====
    let raw_total = (range.rows().count() as u32).saturating_sub(1);
    on_progress(0, raw_total, "parsing");

    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut data_rows: Vec<(usize, Vec<calamine::Data>)> = Vec::new();
    for (i, row) in range.rows().enumerate() {
        if i == 0 { continue; }
        let name = cell_to_string(row.first().unwrap_or(&calamine::Data::Empty));
        if name.trim().is_empty() { continue; }
        if data_rows.len() as u32 >= MAX_ROWS {
            errors.push(format!("超过{}行上限，第{}行及之后被跳过", MAX_ROWS, i + 1));
            break;
        }
        data_rows.push((i + 1, row.to_vec()));
    }
    let total = data_rows.len() as u32;

    // ===== 写入阶段 =====
    let mut imported = 0u32;
    let mut updated = 0u32;
    let mut skipped = 0u32;
    let mut models_created = 0u32;
    let mut processed = 0u32;

    on_progress(0, total, "importing");

    db::with_transaction(conn, |c| {
        for (row_no, row) in &data_rows {
            let row_no = *row_no;
            let cell = |idx: usize| -> String {
                cell_to_string(row.get(idx).unwrap_or(&calamine::Data::Empty)).trim().to_string()
            };

            let name = cell(0);
            let model_name = cell(1);
            let room_name = cell(2);
            let rack_name = cell(3);
            let u_str = cell(4);
            let (start_u, end_u) = parse_u_range(&u_str).unzip();
            let ip_addresses = cell(5);
            let serial_no = cell(6);
            let asset_no = cell(7);
            let department = cell(8);
            let owner = cell(9);
            let function_desc = cell(10);
            let purchase_date_str = cell(11);
            let warranty_expire_str = cell(12);
            let status = match cell(13).as_str() {
                "开机" | "online" => "online",
                "离线" | "offline" => "offline",
                _ => "unconfigured",
            }.to_string();
            let power_watt = cell_to_int(row.get(14).unwrap_or(&calamine::Data::Empty)).unwrap_or(0);

            // N-21：第 16 列（索引 15）设备类型；旧文件缺失 / 空值 → None
            let dt_raw = row.get(15)
                .map(cell_to_string)
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());
            let device_type: Option<&'static str> = dt_raw.as_deref().map(normalize_device_type);
            if let Some(raw) = &dt_raw {
                if device_type == Some("other") && !is_known_other(raw) {
                    warnings.push(format!("第{}行 未知设备类型「{}」已按 其他 处理", row_no, raw));
                }
            }

            let rack_id = if !rack_name.is_empty() {
                Some(db::racks::find_or_create_rack(c, &rack_name)?)
            } else {
                None
            };

            // ===== 重复检测（均自动排除软删） =====
            let mut existing = None;
            if !serial_no.is_empty() {
                existing = db::devices::find_device_by_serial(c, &serial_no)?;
            }
            if existing.is_none() && !asset_no.is_empty() {
                existing = db::devices::find_device_by_asset(c, &asset_no)?;
            }
            if existing.is_none() {
                existing = db::devices::find_device_by_name_in_rack(c, &name, rack_id)?;
            }

            if let Some(existing) = existing {
                if update_mode == "overwrite" {
                    link_rack_room(c, rack_id, &room_name, options.link_room)?;
                    let model_id = resolve_model(c, &model_name, device_type, &mut models_created, &mut warnings)?;
                    let upd = build_overwrite_update(
                        &name, model_id, rack_id, start_u, end_u,
                        &ip_addresses, &serial_no, &asset_no, &department, &owner, &function_desc,
                        &purchase_date_str, &warranty_expire_str, &status, power_watt,
                    );
                    db::devices::update_device(c, existing.id, &upd)?;
                    updated += 1;
                } else {
                    log::warn!("导入跳过第{}行: 记录已存在（id={}）", row_no, existing.id);
                    skipped += 1;
                }
            } else {
                link_rack_room(c, rack_id, &room_name, options.link_room)?;
                let model_id = resolve_model(c, &model_name, device_type, &mut models_created, &mut warnings)?;
                let data = DeviceCreate {
                    name,
                    device_model_id: model_id,
                    rack_id,
                    start_u,
                    end_u,
                    ip_addresses: none_if_empty(ip_addresses),
                    serial_no: none_if_empty(serial_no),
                    asset_no: none_if_empty(asset_no),
                    department: none_if_empty(department),
                    owner: none_if_empty(owner),
                    function: none_if_empty(function_desc),
                    purchase_date: none_if_empty(purchase_date_str),
                    warranty_expire: none_if_empty(warranty_expire_str),
                    status: Some(status),
                    power_watt: if power_watt > 0 { Some(power_watt) } else { None },
                    // 高度由 insert_device 推导：U 位区间优先，其次型号高度（None 不显式指定）
                    height_u: None,
                };
                db::devices::insert_device(c, &data)?;
                imported += 1;
            }

            processed += 1;
            if processed.is_multiple_of(PROGRESS_EVERY) {
                on_progress(processed, total, "importing");
            }
        }
        Ok(())
    })?;

    on_progress(total, total, "done");

    Ok(ImportResult {
        imported,
        updated,
        skipped,
        total,
        models_created,
        warnings,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration;
    use rusqlite::Connection;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        migration::run(&conn).unwrap();
        conn
    }

    /// 生成唯一的临时 xlsx 路径（测试结束即删除，不入库）。
    fn temp_xlsx_path(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mut p = std::env::temp_dir();
        p.push(format!("rackviz_excel_test_{}_{}_{}.xlsx", tag, std::process::id(), nanos));
        p
    }

    /// 用 rust_xlsxwriter 现场生成临时 xlsx（全字符串写入，导入侧 `cell_to_int` 可解析数字文本）。
    fn write_rows(path: &std::path::Path, rows: &[Vec<String>]) {
        let mut wb = Workbook::new();
        {
            let ws = wb.add_worksheet().set_name("Sheet1").unwrap();
            for (r, row) in rows.iter().enumerate() {
                for (c, v) in row.iter().enumerate() {
                    ws.write_string(r as u32, c as u16, v.as_str()).unwrap();
                }
            }
        }
        wb.save(path).unwrap();
    }

    fn header16() -> Vec<String> {
        [
            "设备名称", "型号", "机房", "机柜", "位置(U)",
            "IP地址", "序列号", "资产编号", "使用部门", "责任人",
            "功能描述", "采购日期", "质保到期", "状态", "功率(W)", "设备类型",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn header15() -> Vec<String> {
        header16().into_iter().take(15).collect()
    }

    #[allow(clippy::too_many_arguments)]
    fn make_row(
        name: &str, model: &str, room: &str, rack: &str, pos: &str, ip: &str, sn: &str,
        asset: &str, dept: &str, owner: &str, func: &str, purchase: &str, warranty: &str,
        status: &str, power: &str, dtype: Option<&str>,
    ) -> Vec<String> {
        let mut v: Vec<String> = [name, model, room, rack, pos, ip, sn, asset, dept, owner,
            func, purchase, warranty, status, power]
            .iter()
            .map(|s| s.to_string())
            .collect();
        if let Some(t) = dtype {
            v.push(t.to_string());
        }
        v
    }

    fn default_opts() -> ImportOptions {
        ImportOptions { update_mode: "skip".into(), link_room: false }
    }

    fn noop_progress(_p: u32, _t: u32, _phase: &str) {}

    /// 旧 15 列文件：向后兼容（无第 16 列 → 型号按 'server' 兜底，不报错）。
    #[test]
    fn test_import_legacy_15_columns_backward_compatible() {
        let conn = setup_db();
        let path = temp_xlsx_path("legacy15");
        let rows = vec![
            header15(),
            make_row("Legacy-A", "OldModel", "", "", "", "", "SN-L1", "AS-L1", "", "", "", "", "", "开机", "700", None),
            make_row("Legacy-B", "OldModel", "", "", "", "", "SN-L2", "AS-L2", "", "", "", "", "", "离线", "800", None),
        ];
        write_rows(&path, &rows);

        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 2, "旧 15 列文件应全部导入");
        assert_eq!(res.skipped, 0);
        assert_eq!(res.updated, 0);
        assert_eq!(res.total, 2);
        assert_eq!(res.models_created, 1, "同名型号只应新建一次");
        assert!(res.errors.is_empty());
        assert!(res.warnings.is_empty(), "旧文件无类型列，不应有任何告警");

        // 型号类型按 'server' 兜底
        let models = db::device_models::list_device_models(&conn).unwrap();
        let m = models.iter().find(|m| m.name == "OldModel").unwrap();
        assert_eq!(m.device_type, "server");
        assert_eq!(db::devices::list_devices(&conn, None, None).unwrap().len(), 2);
    }

    /// 第 16 列：中文名 / 英文枚举 / 大小写 / 空值 / 未知值 的型号关联与计数。
    #[test]
    fn test_import_device_type_mapping_and_warnings() {
        let conn = setup_db();
        let path = temp_xlsx_path("types");
        let rows = vec![
            header16(),
            make_row("D1", "M-SwitchCN", "", "", "", "", "SN-T1", "AS-T1", "", "", "", "", "", "开机", "100", Some("交换机")),
            make_row("D2", "M-SwitchEN", "", "", "", "", "SN-T2", "AS-T2", "", "", "", "", "", "开机", "100", Some("switch")),
            make_row("D3", "M-LbUpper", "", "", "", "", "SN-T3", "AS-T3", "", "", "", "", "", "开机", "100", Some("LOADBALANCER")),
            make_row("D4", "M-Empty", "", "", "", "", "SN-T4", "AS-T4", "", "", "", "", "", "开机", "100", Some("")),
            make_row("D5", "M-Unknown", "", "", "", "", "SN-T5", "AS-T5", "", "", "", "", "", "开机", "100", Some("防火墙")),
        ];
        write_rows(&path, &rows);

        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 5);
        assert_eq!(res.total, 5);
        assert_eq!(res.models_created, 5, "5 个不同型号名 → 5 次新建");
        assert_eq!(res.warnings.len(), 1, "仅未知值「防火墙」应告警");
        assert!(res.warnings[0].contains("防火墙") && res.warnings[0].contains("其他"));

        let models = db::device_models::list_device_models(&conn).unwrap();
        let ty = |name: &str| models.iter().find(|m| m.name == name).unwrap().device_type.clone();
        assert_eq!(ty("M-SwitchCN"), "switch");
        assert_eq!(ty("M-SwitchEN"), "switch");
        assert_eq!(ty("M-LbUpper"), "loadbalancer");
        assert_eq!(ty("M-Empty"), "server", "空类型列 → 'server' 兜底");
        assert_eq!(ty("M-Unknown"), "other", "未知值 → other 兜底");
    }

    /// update_mode = "skip"：重复记录跳过。
    #[test]
    fn test_import_update_mode_skip() {
        let conn = setup_db();
        let path = temp_xlsx_path("skip");
        let rows = vec![
            header16(),
            make_row("Skip-1", "SModel", "", "", "", "", "SN-SKIP", "AS-SKIP", "", "", "", "", "", "开机", "10", Some("服务器")),
        ];
        write_rows(&path, &rows);

        let first = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        assert_eq!(first.imported, 1);

        // 第二次导入（skip）→ 跳过
        let opts = ImportOptions { update_mode: "skip".into(), link_room: false };
        let second = import_devices_excel(&conn, path.to_str().unwrap(), &opts, &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(second.imported, 0);
        assert_eq!(second.updated, 0);
        assert_eq!(second.skipped, 1);
        assert_eq!(second.total, 1);
        assert_eq!(db::devices::list_devices(&conn, None, None).unwrap().len(), 1, "不得重复插入");
    }

    /// update_mode = "overwrite"：命中已存在记录时更新并计入 updated。
    #[test]
    fn test_import_update_mode_overwrite() {
        let conn = setup_db();
        let path = temp_xlsx_path("overwrite");
        let first_rows = vec![
            header16(),
            make_row("OW-1", "OWModel", "", "", "", "", "SN-OW", "AS-OW", "旧部门", "张三", "", "", "", "开机", "10", Some("服务器")),
        ];
        write_rows(&path, &first_rows);
        let first = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        assert_eq!(first.imported, 1);

        // 同序列号，改动部门与状态，overwrite
        let second_rows = vec![
            header16(),
            make_row("OW-1", "OWModel", "", "", "", "", "SN-OW", "AS-OW", "新部门", "李四", "", "", "", "离线", "10", Some("服务器")),
        ];
        write_rows(&path, &second_rows);
        let opts = ImportOptions { update_mode: "overwrite".into(), link_room: false };
        let second = import_devices_excel(&conn, path.to_str().unwrap(), &opts, &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(second.imported, 0);
        assert_eq!(second.updated, 1);
        assert_eq!(second.skipped, 0);

        let devices = db::devices::list_devices(&conn, None, None).unwrap();
        assert_eq!(devices.len(), 1, "overwrite 不得新增记录");
        assert_eq!(devices[0].department, "新部门");
        assert_eq!(devices[0].owner, "李四");
        assert_eq!(devices[0].status, "offline");
    }

    /// 重复检测排除软删记录：软删后同序列号应被视为"新记录"。
    #[test]
    fn test_import_duplicate_detection_excludes_soft_deleted() {
        let conn = setup_db();
        // 先建一条 SN-DEL 设备并软删
        let ghost = db::devices::insert_device(&conn, &DeviceCreate {
            name: "Ghost".into(),
            serial_no: Some("SN-DEL".into()),
            ..Default::default()
        }).unwrap();
        assert!(db::devices::soft_delete_device(&conn, ghost.id).unwrap());

        let path = temp_xlsx_path("softdel");
        let rows = vec![
            header16(),
            make_row("Reborn", "RBModel", "", "", "", "", "SN-DEL", "AS-DEL", "", "", "", "", "", "开机", "5", Some("服务器")),
        ];
        write_rows(&path, &rows);
        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 1, "软删记录不参与查重 → 应作为新记录导入");
        assert_eq!(res.skipped, 0);
        assert_eq!(db::devices::list_devices(&conn, None, None).unwrap().len(), 1);
    }

    /// link_room：机房列非空且 link_room=true → 幂等建机房并写回机柜归属。
    #[test]
    fn test_import_link_room_creates_room_and_links_rack() {
        let conn = setup_db();
        let path = temp_xlsx_path("linkroom");
        let rows = vec![
            header16(),
            make_row("LR-1", "LRModel", "机房Z", "RACK-Z", "", "", "SN-LR1", "AS-LR1", "", "", "", "", "", "开机", "10", Some("服务器")),
            make_row("LR-2", "LRModel", "机房Z", "RACK-Z", "", "", "SN-LR2", "AS-LR2", "", "", "", "", "", "开机", "10", Some("服务器")),
        ];
        write_rows(&path, &rows);
        let opts = ImportOptions { update_mode: "skip".into(), link_room: true };
        let res = import_devices_excel(&conn, path.to_str().unwrap(), &opts, &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 2);
        let rooms = db::rooms::list_rooms(&conn).unwrap();
        assert_eq!(rooms.len(), 1, "同名机房幂等，只建一次");
        assert_eq!(rooms[0].name, "机房Z");

        let racks = db::racks::list_racks(&conn).unwrap();
        let rz = racks.iter().find(|r| r.name == "RACK-Z").unwrap();
        assert_eq!(rz.room_id, Some(rooms[0].id), "机柜应归属到导入的机房");
    }

    /// link_room=false 时不得创建机房。
    #[test]
    fn test_import_link_room_false_does_not_create_room() {
        let conn = setup_db();
        let path = temp_xlsx_path("nolinkroom");
        let rows = vec![
            header16(),
            make_row("NL-1", "NLModel", "机房Y", "RACK-Y", "", "", "SN-NL1", "AS-NL1", "", "", "", "", "", "开机", "10", Some("服务器")),
        ];
        write_rows(&path, &rows);
        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 1);
        assert!(db::rooms::list_rooms(&conn).unwrap().is_empty(), "未开启关联机房不得建机房");
        let racks = db::racks::list_racks(&conn).unwrap();
        assert_eq!(racks[0].room_id, None);
    }

    /// 同名型号已存在且传入类型不同 → 复用 id、不覆盖 type 并告警。
    #[test]
    fn test_import_existing_model_type_not_overwritten() {
        let conn = setup_db();
        // 预置型号 Twin，用户手工设为 router
        let twin_id = db::device_models::find_or_create_model(&conn, "Twin", Some("router")).unwrap();

        let path = temp_xlsx_path("modelsync");
        let rows = vec![
            header16(),
            make_row("TwinDev", "Twin", "", "", "", "", "SN-TW", "AS-TW", "", "", "", "", "", "开机", "10", Some("交换机")),
        ];
        write_rows(&path, &rows);
        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 1);
        assert_eq!(res.models_created, 0, "同名型号不得重复新建");
        let m = db::device_models::get_device_model(&conn, twin_id).unwrap().unwrap();
        assert_eq!(m.device_type, "router", "既有 type 不得被覆盖");
        assert_eq!(res.warnings.len(), 1);
        assert!(res.warnings[0].contains("Twin") && res.warnings[0].contains("路由器"));
    }

    /// 同名型号已存在且类型一致 → 不产生告警（避免重复导入噪声）。
    #[test]
    fn test_import_existing_model_same_type_no_warning() {
        let conn = setup_db();
        db::device_models::find_or_create_model(&conn, "Same", Some("switch")).unwrap();
        let path = temp_xlsx_path("modelsame");
        let rows = vec![
            header16(),
            make_row("SameDev", "Same", "", "", "", "", "SN-SA", "AS-SA", "", "", "", "", "", "开机", "10", Some("switch")),
        ];
        write_rows(&path, &rows);
        let res = import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(res.models_created, 0);
        assert!(res.warnings.is_empty(), "类型一致时不应告警");
    }

    /// 进度回调：至少覆盖 parsing / importing / done 三个阶段。
    #[test]
    fn test_import_progress_phases() {
        let conn = setup_db();
        let path = temp_xlsx_path("progress");
        let rows = vec![
            header16(),
            make_row("P1", "PModel", "", "", "", "", "SN-P1", "AS-P1", "", "", "", "", "", "开机", "10", Some("服务器")),
        ];
        write_rows(&path, &rows);

        let phases = std::cell::RefCell::new(Vec::<String>::new());
        let cb = |_p: u32, _t: u32, phase: &str| {
            phases.borrow_mut().push(phase.to_string());
        };
        import_devices_excel(&conn, path.to_str().unwrap(), &default_opts(), &cb).unwrap();
        std::fs::remove_file(&path).ok();

        let p = phases.borrow();
        assert!(p.iter().any(|x| x == "parsing"), "应上报 parsing 阶段");
        assert!(p.iter().any(|x| x == "importing"), "应上报 importing 阶段");
        assert_eq!(p.last().map(|s| s.as_str()), Some("done"), "应以 done 结束");
    }

    /// N-21 往返保真：export → import 后 `device_models.type` 不丢失。
    #[test]
    fn test_export_import_roundtrip_preserves_model_type() {
        // 源库：型号 RouterModel 类型 switch + 一台挂该型号的设备
        let src = setup_db();
        let mid = db::device_models::find_or_create_model(&src, "RT-Model", Some("switch")).unwrap();
        db::devices::insert_device(&src, &DeviceCreate {
            name: "RT-Dev".into(),
            device_model_id: Some(mid),
            ip_addresses: Some("10.0.0.1".into()),
            ..Default::default()
        }).unwrap();

        let buf = export_devices_data_excel(&src).unwrap();
        let path = temp_xlsx_path("roundtrip");
        std::fs::write(&path, buf).unwrap();

        // 目标库：全新，导入导出文件
        let dst = setup_db();
        let res = import_devices_excel(&dst, path.to_str().unwrap(), &default_opts(), &noop_progress).unwrap();
        std::fs::remove_file(&path).ok();

        assert_eq!(res.imported, 1);
        let models = db::device_models::list_device_models(&dst).unwrap();
        let m = models.iter().find(|m| m.name == "RT-Model").unwrap();
        assert_eq!(m.device_type, "switch", "往返后型号类型必须保留");
    }

    /// 导出内容基础健全性：含 16 列表头且首行为冻结/自动筛选已设置（不 panic）。
    #[test]
    fn test_export_devices_data_excel_smoke() {
        let conn = setup_db();
        let buf = export_devices_data_excel(&conn).unwrap();
        assert!(!buf.is_empty(), "空库导出仍应产出 xlsx 字节流");
        let buf2 = export_racks_excel(&conn).unwrap();
        assert!(!buf2.is_empty());
    }

    // ==================== Stage 2A 对抗性验证（QA 补充） ====================

    /// 读取 xlsx 全部工作表的所有单元格文本（用于断言导出内容）。
    fn read_all_cells(path: &std::path::Path) -> Vec<String> {
        use calamine::{open_workbook, Reader, Xlsx};
        let mut wb: Xlsx<_> = open_workbook(path).unwrap();
        let mut out = Vec::new();
        for name in wb.sheet_names().to_owned() {
            if let Ok(range) = wb.worksheet_range(&name) {
                for row in range.rows() {
                    for c in row {
                        out.push(cell_to_string(c));
                    }
                }
            }
        }
        out
    }

    /// 读取机柜部署图导出中「使用率」行的第 1 个机柜数值（百分比）。
    fn read_usage_row(path: &std::path::Path) -> f64 {
        use calamine::{open_workbook, Reader, Xlsx};
        let mut wb: Xlsx<_> = open_workbook(path).unwrap();
        let range = wb.worksheet_range("机柜部署图").unwrap();
        for row in range.rows() {
            let first = row.first().map(cell_to_string).unwrap_or_default();
            if first == "使用率" {
                return row.get(1).map(|c| match c {
                    calamine::Data::Float(f) => *f,
                    calamine::Data::Int(i) => *i as f64,
                    _ => 0.0,
                }).unwrap_or(0.0);
            }
        }
        panic!("导出文件中未找到「使用率」行");
    }

    /// 点 1：设备台账导出必须排除软删设备（构造「软删设备名」不出现在导出内容中）。
    #[test]
    fn test_export_devices_data_excludes_soft_deleted() {
        let conn = setup_db();
        db::devices::insert_device(&conn, &DeviceCreate { name: "KEEP-EXP".into(), ..Default::default() }).unwrap();
        let gone = db::devices::insert_device(&conn, &DeviceCreate { name: "GONE-EXP".into(), ..Default::default() }).unwrap();
        db::devices::soft_delete_device(&conn, gone.id).unwrap();

        let buf = export_devices_data_excel(&conn).unwrap();
        let path = temp_xlsx_path("exp_softdel");
        std::fs::write(&path, buf).unwrap();
        let cells = read_all_cells(&path);
        std::fs::remove_file(&path).ok();

        assert!(cells.iter().any(|c| c == "KEEP-EXP"), "在用设备应出现在导出");
        assert!(!cells.iter().any(|c| c == "GONE-EXP"), "软删设备不得出现在导出");
    }

    /// 点 1：机柜部署图导出的「使用率」不得计入软删设备占用的 U 位。
    #[test]
    fn test_export_racks_usage_excludes_soft_deleted() {
        let conn = setup_db();
        db::racks::insert_rack(&conn, &crate::models::RackCreate {
            name: "RX".into(), height_u: Some(10), ..Default::default()
        }).unwrap();
        let dev = db::devices::insert_device(&conn, &DeviceCreate {
            name: "OCC".into(), rack_id: Some(1), start_u: Some(1), end_u: Some(4),
            ..Default::default()
        }).unwrap();

        // 未软删：使用率 4/10 = 40%
        let buf = export_racks_excel(&conn).unwrap();
        let p1 = temp_xlsx_path("rack_active");
        std::fs::write(&p1, &buf).unwrap();
        let usage_active = read_usage_row(&p1);
        std::fs::remove_file(&p1).ok();
        assert!((usage_active - 40.0).abs() < 0.01, "在用设备：使用率应为 40%，实际 {}", usage_active);

        // 软删后：使用率应为 0%（不得计入占用）
        db::devices::soft_delete_device(&conn, dev.id).unwrap();
        let buf2 = export_racks_excel(&conn).unwrap();
        let p2 = temp_xlsx_path("rack_deleted");
        std::fs::write(&p2, &buf2).unwrap();
        let usage_deleted = read_usage_row(&p2);
        std::fs::remove_file(&p2).ok();
        assert!(usage_deleted.abs() < 0.01, "软删后使用率应为 0%，实际 {}", usage_deleted);
    }
}
