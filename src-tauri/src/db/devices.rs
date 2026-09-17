use rusqlite::{Connection, params};
use rusqlite::types::ToSql;
use rusqlite::OptionalExtension;
use chrono::{DateTime, NaiveDate, Utc};
use crate::models::*;
use crate::error::AppError;
use crate::db::{patch_assign, NOT_DELETED, now_iso};

/// 转义 LIKE 模式中的通配符，防止用户输入 `%`/`_`/`\` 干扰匹配。
/// SQL 侧必须配套 `ESCAPE '\'`。
fn escape_like(s: &str) -> String {
    s.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_")
}

fn row_to_device(row: &rusqlite::Row) -> rusqlite::Result<Device> {
    Ok(Device {
        id: row.get(0)?,
        name: row.get(1)?,
        device_model_id: row.get(2)?,
        rack_id: row.get(3)?,
        start_u: row.get(4)?,
        end_u: row.get(5)?,
        ip_addresses: row.get::<_, String>(6).unwrap_or_default(),
        serial_no: row.get::<_, String>(7).unwrap_or_default(),
        asset_no: row.get::<_, String>(8).unwrap_or_default(),
        department: row.get::<_, String>(9).unwrap_or_default(),
        owner: row.get::<_, String>(10).unwrap_or_default(),
        function: row.get::<_, String>(11).unwrap_or_default(),
        purchase_date: row.get(12)?,
        warranty_expire: row.get(13)?,
        status: row.get::<_, String>(14).unwrap_or_else(|_| "unconfigured".into()),
        power_watt: row.get(15).unwrap_or(0),
        height_u: row.get::<_, i32>(16).unwrap_or(1),
        created_at: row.get(17)?,
        updated_at: row.get(18)?,
        deleted_at: row.get(19)?,
    })
}

const DEVICE_SELECT: &str =
    "SELECT id, name, device_model_id, rack_id, start_u, end_u, \
     ip_addresses, serial_no, asset_no, department, owner, \
     function, purchase_date, warranty_expire, status, power_watt, height_u, \
     created_at, updated_at, deleted_at \
     FROM devices";

/// 推导设备固有高度：显式值 > 已给 U 位区间 > 型号高度 > 1。
fn resolve_device_height(
    conn: &Connection,
    explicit: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
    device_model_id: Option<i32>,
) -> Result<i32, AppError> {
    if let Some(h) = explicit {
        if h >= 1 {
            return Ok(h);
        }
    }
    if let (Some(s), Some(e)) = (start_u, end_u) {
        if e >= s {
            return Ok(e - s + 1);
        }
    }
    if let Some(mid) = device_model_id {
        let h: Option<i32> = conn
            .query_row("SELECT height_u FROM device_models WHERE id = ?1", params![mid], |r| r.get(0))
            .optional()?;
        if let Some(h) = h.filter(|h| *h >= 1) {
            return Ok(h);
        }
    }
    Ok(1)
}

/// 回收站可恢复窗口（天）：超过该天数的软删记录拒绝恢复（§9 Q6，固定常量，不物理清理）。
const RESTORE_WINDOW_DAYS: i64 = 30;

/// `query_devices` 分页默认/上限（防止前端传超大 limit 拖垮 UI）。
const QUERY_DEFAULT_LIMIT: i64 = 100;
const QUERY_MAX_LIMIT: i64 = 1000;

/// 全量列出设备（RackView / 导出 / 报表用）。
///
/// **保留全量语义**：恒追加 `deleted_at IS NULL`（§10.2 第 8 项裁决，不暴露 `include_deleted`）。
pub fn list_devices(conn: &Connection, rack_id: Option<i32>, search: Option<String>) -> Result<Vec<Device>, AppError> {
    let mut conditions: Vec<String> = vec![NOT_DELETED.to_string()];
    let mut param_values: Vec<String> = Vec::new();

    if let Some(rid) = rack_id {
        conditions.push(format!("rack_id = ?{}", param_values.len() + 1));
        param_values.push(rid.to_string());
    }
    if let Some(ref s) = search {
        if !s.trim().is_empty() {
            // B1 全局搜索：五字段同参（与 query_devices 口径一致）；
            // 通配符转义 + ESCAPE '\'（审查红线 S-2 / D-1）
            param_values.push(format!("%{}%", escape_like(s)));
            let idx = param_values.len();
            conditions.push(format!(
                "(name LIKE ?{idx} ESCAPE '\\' \
                 OR ip_addresses LIKE ?{idx} ESCAPE '\\' \
                 OR serial_no LIKE ?{idx} ESCAPE '\\' \
                 OR asset_no LIKE ?{idx} ESCAPE '\\' \
                 OR owner LIKE ?{idx} ESCAPE '\\')"
            ));
        }
    }

    let sql = format!(
        "{} WHERE {} ORDER BY name",
        DEVICE_SELECT,
        conditions.join(" AND ")
    );

    let mut stmt = conn.prepare(&sql)?;
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = param_values
        .iter()
        .map(|s| s as &dyn rusqlite::types::ToSql)
        .collect();

    let rows = stmt.query_map(params_refs.as_slice(), row_to_device)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

/// 服务端分页 + 多字段搜索 + 白名单排序（N-01/N-02）。
///
/// 返回 `(items, total)`：`total` 为**同一 WHERE 条件下**的 `COUNT(*)`。
///
/// - 过滤：默认 `deleted_at IS NULL`；`include_deleted=true` 时才放行；
///   `rack_id = ?`；`room_id` 经 `rack_id IN (SELECT id FROM racks WHERE room_id = ?)`。
/// - 搜索：**五字段同参**（B1 全局搜索）——
///   `(name LIKE ? OR ip_addresses LIKE ? OR serial_no LIKE ? OR asset_no LIKE ? OR owner LIKE ?)`，
///   复用 `escape_like` 并带 `ESCAPE '\'`。
/// - 排序：**白名单映射**字段 → 列名；`sort_order` 仅 `asc|desc`；**绝不允许前端字符串裸拼接**。
/// - 分页：`LIMIT ? OFFSET ?`。
pub fn query_devices(conn: &Connection, q: &DeviceQuery) -> Result<(Vec<Device>, i64), AppError> {
    let include_deleted = q.include_deleted.unwrap_or(false);

    let mut where_parts: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    // 1) 软删除过滤（默认排除）
    if !include_deleted {
        where_parts.push(NOT_DELETED.to_string());
    }
    // 2) 机柜过滤
    if let Some(rid) = q.rack_id {
        params.push(Box::new(rid));
        where_parts.push(format!("rack_id = ?{}", params.len()));
    }
    // 3) 机房过滤（经机柜表）
    if let Some(room_id) = q.room_id {
        params.push(Box::new(room_id));
        where_parts.push(format!(
            "rack_id IN (SELECT id FROM racks WHERE room_id = ?{})",
            params.len()
        ));
    }
    // 4) 多字段搜索（五字段同参：B1 全局搜索）
    if let Some(ref s) = q.search {
        let s = s.trim();
        if !s.is_empty() {
            params.push(Box::new(format!("%{}%", escape_like(s))));
            let idx = params.len();
            where_parts.push(format!(
                "(name LIKE ?{idx} ESCAPE '\\' \
                 OR ip_addresses LIKE ?{idx} ESCAPE '\\' \
                 OR serial_no LIKE ?{idx} ESCAPE '\\' \
                 OR asset_no LIKE ?{idx} ESCAPE '\\' \
                 OR owner LIKE ?{idx} ESCAPE '\\')"
            ));
        }
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", where_parts.join(" AND "))
    };

    // total：同 WHERE 的 COUNT(*)
    let count_sql = format!("SELECT COUNT(*) FROM devices{}", where_clause);
    let count_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let total: i64 = conn.query_row(&count_sql, count_refs.as_slice(), |r| r.get(0))?;

    // 排序白名单（仅映射，不做任何未校验拼接）
    let order_col = match q.sort_field.as_deref() {
        Some("ip_addresses") => "ip_addresses",
        Some("serial_no") => "serial_no",
        Some("asset_no") => "asset_no",
        Some("status") => "status",
        Some("power_watt") => "power_watt",
        Some("created_at") => "created_at",
        Some("updated_at") => "updated_at",
        Some("rack_id") => "rack_id",
        Some("name") => "name",
        _ => "name",
    };
    let order_dir = match q.sort_order.as_deref() {
        Some("desc") => "DESC",
        _ => "ASC",
    };

    // 分页参数（夹取到安全范围）
    let limit = q.limit.unwrap_or(QUERY_DEFAULT_LIMIT).clamp(1, QUERY_MAX_LIMIT);
    let offset = q.offset.unwrap_or(0).max(0);

    let list_sql = format!(
        "{}{} ORDER BY {} {} LIMIT ?{} OFFSET ?{}",
        DEVICE_SELECT,
        where_clause,
        order_col,
        order_dir,
        params.len() + 1,
        params.len() + 2
    );

    let mut stmt = conn.prepare(&list_sql)?;
    let mut list_refs: Vec<&dyn ToSql> = params.iter().map(|p| p.as_ref()).collect();
    list_refs.push(&limit);
    list_refs.push(&offset);
    let rows = stmt.query_map(list_refs.as_slice(), row_to_device)?;
    let items = rows.collect::<Result<Vec<_>, _>>().map_err(AppError::from)?;

    Ok((items, total))
}

/// 回收站列表：仅软删除记录，按 `deleted_at DESC`（N-09）。
pub fn list_deleted_devices(conn: &Connection, limit: Option<i64>) -> Result<Vec<Device>, AppError> {
    let limit = limit.unwrap_or(500).clamp(1, 5000);
    let sql = format!(
        "{} WHERE deleted_at IS NOT NULL ORDER BY deleted_at DESC LIMIT ?1",
        DEVICE_SELECT
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![limit], row_to_device)?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.into())
}

pub fn get_device(conn: &Connection, id: i32) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!("{} WHERE id = ?1", DEVICE_SELECT))?;
    let mut rows = stmt.query_map(params![id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

fn parse_optional_date(s: &Option<String>) -> Option<NaiveDate> {
    s.as_deref().and_then(|v| {
        if v.trim().is_empty() { return None; }
        NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d")
            .or_else(|_| NaiveDate::parse_from_str(v.trim(), "%Y/%m/%d"))
            .or_else(|_| NaiveDate::parse_from_str(v.trim(), "%Y.%m.%d"))
            .ok()
    })
}

// ============================================================================
// N-10 输入验证增强（2026-09-16）：后端为最终防线，覆盖表单 / Excel 导入 / 拖拽全部写入路径
//
// 规则与前端 `DeviceFormModal.tsx` 对齐（前后端双校验）：
// - U 位区间：end ≥ start，且任意 U 位 ≥ 1
// - U 位边界：双边齐全时须落在所属机柜 height_u 内
// - U 位重叠：同机柜 active 设备区间互斥（排除自身；软删设备不参与——v5 回收站语义）
// - IP 列表：分隔符 `, ， ; ； 空白`，每项为无前导零 IPv4 或含 `:`（放行 IPv6）；允许空
// - status：枚举白名单（与前端 DEVICE_STATUSES / 导入归一化一致）
// ============================================================================

/// 设备状态枚举白名单
const DEVICE_STATUS_VALUES: [&str; 3] = ["online", "offline", "unconfigured"];

/// U 位区间校验：双边给出时 end ≥ start；任一给出时值 ≥ 1（单边给值不拒绝，兼容既有语义）
fn validate_u_range(start_u: Option<i32>, end_u: Option<i32>) -> Result<(), AppError> {
    for v in [start_u, end_u].into_iter().flatten() {
        if v < 1 {
            return Err(AppError::validation(&format!("U 位({v})无效：U 位从 1 开始")));
        }
    }
    if let (Some(s), Some(e)) = (start_u, end_u) {
        if e < s {
            return Err(AppError::validation(&format!(
                "结束U位({e})不能小于起始U位({s})"
            )));
        }
    }
    Ok(())
}

/// U 位边界校验：双边齐全且有机柜时，须落在机柜高度内（顺带验证机柜存在性）
fn validate_u_boundary(
    conn: &Connection,
    rack_id: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
) -> Result<(), AppError> {
    let (Some(rid), Some(_s), Some(e)) = (rack_id, start_u, end_u) else {
        return Ok(());
    };
    let row: Option<(String, i32)> = conn
        .query_row(
            "SELECT name, height_u FROM racks WHERE id = ?1",
            params![rid],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((name, height)) = row else {
        return Err(AppError::validation(&format!(
            "所属机柜不存在（id={rid}），无法安置 U 位"
        )));
    };
    if e > height {
        return Err(AppError::validation(&format!(
            "结束U位({e})超出机柜「{name}」高度({height}U)"
        )));
    }
    Ok(())
}

/// U 位重叠校验：同机柜内与任一其它 active 设备的 U 位区间相交即冲突（排除自身 / 软删）。
/// `exclude_id = None` 用于新增场景。
fn validate_u_overlap(
    conn: &Connection,
    exclude_id: Option<i32>,
    rack_id: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
) -> Result<(), AppError> {
    let (Some(rid), Some(s), Some(e)) = (rack_id, start_u, end_u) else {
        return Ok(());
    };
    // exclude_id 用 -1（不存在的 id）表示"不排除"，避免动态 SQL 拼参
    let exclude = exclude_id.unwrap_or(-1);
    let hit: Option<(String, i32, i32)> = conn
        .query_row(
            "SELECT name, start_u, end_u FROM devices \
             WHERE rack_id = ?1 AND deleted_at IS NULL \
             AND start_u IS NOT NULL AND end_u IS NOT NULL \
             AND start_u <= ?2 AND ?3 <= end_u AND id != ?4 \
             ORDER BY id LIMIT 1",
            params![rid, e, s, exclude],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .optional()?;
    if let Some((name, s2, e2)) = hit {
        return Err(AppError::conflict(&format!(
            "U 位 {s}-{e} 与设备「{name}」(U{s2}-{e2}) 重叠，请更换位置"
        )));
    }
    Ok(())
}

/// 无前导零 IPv4：与前端 `IPV4_RE`（`25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d`）等价——
/// 4 段、每段 1~3 位纯数字、无前导零、数值 ≤ 255。
fn is_valid_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    parts.len() == 4
        && parts.iter().all(|p| {
            !p.is_empty()
                && p.len() <= 3
                && p.bytes().all(|b| b.is_ascii_digit())
                && !(p.len() > 1 && p.starts_with('0'))
                && p.parse::<u16>().map(|v| v <= 255).unwrap_or(false)
        })
}

/// IP 列表校验：与前端 `isValidIpList` 同一规则——
/// 分隔符为 `, ， ; ；` 及空白（覆盖前端 `\s` 的常见形态）；空 / 全空白放行；
/// 每项为 IPv4 或含 `:`（IPv6 放行，与前端一致从宽）。
fn validate_ip_addresses(raw: &str) -> Result<(), AppError> {
    let parts: Vec<&str> = raw
        .split([',', '，', ';', '；', ' ', '\t', '\r', '\n'])
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if parts.is_empty() {
        return Ok(());
    }
    for p in parts {
        if !(is_valid_ipv4(p) || p.contains(':')) {
            return Err(AppError::validation(&format!(
                "IP 地址格式不正确：{p}（多个 IP 用逗号分隔）"
            )));
        }
    }
    Ok(())
}

/// status 枚举校验（白名单；空串同样拒绝，不允许静默写脏）
fn validate_status(status: &str) -> Result<(), AppError> {
    if DEVICE_STATUS_VALUES.contains(&status) {
        Ok(())
    } else {
        Err(AppError::validation(&format!(
            "设备状态取值非法：{status:?}（须为 online / offline / unconfigured）"
        )))
    }
}

/// 设备写入校验统一入口（insert / update 共用）。
/// `exclude_id`：update 场景传设备自身 id 以排除重叠自查；insert 传 None。
pub fn validate_device_input(
    conn: &Connection,
    exclude_id: Option<i32>,
    rack_id: Option<i32>,
    start_u: Option<i32>,
    end_u: Option<i32>,
    ip_addresses: Option<&str>,
    status: &str,
) -> Result<(), AppError> {
    validate_u_range(start_u, end_u)?;
    validate_u_boundary(conn, rack_id, start_u, end_u)?;
    validate_u_overlap(conn, exclude_id, rack_id, start_u, end_u)?;
    if let Some(ip) = ip_addresses {
        validate_ip_addresses(ip)?;
    }
    validate_status(status)?;
    Ok(())
}

pub fn insert_device(conn: &Connection, data: &DeviceCreate) -> Result<Device, AppError> {
    let status = data.status.as_deref().unwrap_or("unconfigured");
    // N-10：写入前校验（表单 / 导入 / 拖拽共用此函数，此处为最终防线）
    validate_device_input(
        conn,
        None,
        data.rack_id,
        data.start_u,
        data.end_u,
        data.ip_addresses.as_deref(),
        status,
    )?;
    let purchase_date = parse_optional_date(&data.purchase_date);
    let warranty_expire = parse_optional_date(&data.warranty_expire);
    let height_u = resolve_device_height(conn, data.height_u, data.start_u, data.end_u, data.device_model_id)?;
    // 时间戳单一维护方（§8-1）：insert 时 created_at = updated_at = now
    let now = now_iso();

    conn.execute(
        "INSERT INTO devices (name, device_model_id, rack_id, start_u, end_u, \
         ip_addresses, serial_no, asset_no, department, owner, function, \
         purchase_date, warranty_expire, status, power_watt, height_u, \
         created_at, updated_at) \
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
        params![
            data.name,
            data.device_model_id,
            data.rack_id,
            data.start_u,
            data.end_u,
            data.ip_addresses.as_deref().unwrap_or(""),
            data.serial_no.as_deref().unwrap_or(""),
            data.asset_no.as_deref().unwrap_or(""),
            data.department.as_deref().unwrap_or(""),
            data.owner.as_deref().unwrap_or(""),
            data.function.as_deref().unwrap_or(""),
            purchase_date,
            warranty_expire,
            status,
            data.power_watt.unwrap_or(0),
            height_u,
            now.as_str(),
            now.as_str(),
        ],
    )?;
    let id = conn.last_insert_rowid() as i32;
    get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))
}

pub fn update_device(conn: &Connection, id: i32, data: &DeviceUpdate) -> Result<Option<Device>, AppError> {
    let existing = get_device(conn, id)?;
    let Some(cur) = existing else {
        return Ok(None);
    };

    // 显式字段列表动态 SQL：Unset 跳过、Set 赋值、Clear 置 NULL（替代 COALESCE）
    let mut assignments: Vec<String> = Vec::new();
    let mut params: Vec<Box<dyn ToSql>> = Vec::new();

    patch_assign!(assignments, params, "name", &data.name);
    patch_assign!(assignments, params, "device_model_id", &data.device_model_id);
    patch_assign!(assignments, params, "rack_id", &data.rack_id);
    patch_assign!(assignments, params, "start_u", &data.start_u);
    patch_assign!(assignments, params, "end_u", &data.end_u);
    patch_assign!(assignments, params, "ip_addresses", &data.ip_addresses);
    patch_assign!(assignments, params, "serial_no", &data.serial_no);
    patch_assign!(assignments, params, "asset_no", &data.asset_no);
    patch_assign!(assignments, params, "department", &data.department);
    patch_assign!(assignments, params, "owner", &data.owner);
    patch_assign!(assignments, params, "function", &data.function);
    patch_assign!(assignments, params, "status", &data.status);
    patch_assign!(assignments, params, "power_watt", &data.power_watt);

    // N-10：字段级校验——仅对显式 Set 的新值校验（Unset 保留旧值当初已验；Clear 置 NULL 合法）
    if let Patch::Set(ip) = &data.ip_addresses {
        validate_ip_addresses(ip)?;
    }
    if let Patch::Set(st) = &data.status {
        validate_status(st)?;
    }

    // 合并 Patch 三态得更新后实际 U 位三要素（供 height 同步与 N-10 空间校验共用）
    let eff_rack = match data.rack_id {
        Patch::Set(v) => Some(v),
        Patch::Clear => None,
        Patch::Unset => cur.rack_id,
    };
    let eff_start = match data.start_u {
        Patch::Set(v) => Some(v),
        Patch::Clear => None,
        Patch::Unset => cur.start_u,
    };
    let eff_end = match data.end_u {
        Patch::Set(v) => Some(v),
        Patch::Clear => None,
        Patch::Unset => cur.end_u,
    };
    // 空间校验（区间/边界/重叠）仅在 U 位三要素任一实际变更时执行：
    // 历史脏数据的非空间编辑（如改名）不受阻，动 U 位时才要求解决冲突
    if !matches!(data.rack_id, Patch::Unset)
        || !matches!(data.start_u, Patch::Unset)
        || !matches!(data.end_u, Patch::Unset)
    {
        validate_u_range(eff_start, eff_end)?;
        validate_u_boundary(conn, eff_rack, eff_start, eff_end)?;
        validate_u_overlap(conn, Some(id), eff_rack, eff_start, eff_end)?;
    }

    // 日期字段：值转 NaiveDate 存储；Clear 或空串 → NULL
    push_date_assignment(&mut assignments, &mut params, "purchase_date", &data.purchase_date)?;
    push_date_assignment(&mut assignments, &mut params, "warranty_expire", &data.warranty_expire)?;

    // 固有高度 height_u（NOT NULL，不允许 Clear）：
    // - Set(v) → 显式写值（v ≥ 1，否则报校验错）；
    // - Unset → 若本次更新使设备落在有效 U 位区间（start/end 皆非空且 end≥start），
    //   自动同步为区间高度（重上架/换位置时保证高度不丢）；未落在区间则保持原值（下架保留高度）。
    match &data.height_u {
        Patch::Clear => {
            return Err(AppError::validation("设备固有高度不能置空"));
        }
        Patch::Set(v) if *v < 1 => {
            return Err(AppError::validation(&format!("设备固有高度无效: {}（须 ≥ 1）", v)));
        }
        Patch::Set(v) => {
            assignments.push(format!("height_u = ?{}", params.len() + 1));
            params.push(Box::new(*v));
        }
        Patch::Unset => {
            // 复用上方合并所得 eff_start/eff_end：更新后落在有效 U 位区间时
            // 自动同步为区间高度（重上架/换位置时保证高度不丢）；否则保持原值（下架保留高度）
            if let (Some(s), Some(e)) = (eff_start, eff_end) {
                if e >= s && e - s + 1 != cur.height_u {
                    assignments.push(format!("height_u = ?{}", params.len() + 1));
                    params.push(Box::new(e - s + 1));
                }
            }
        }
    }

    if assignments.is_empty() {
        return Ok(Some(cur));
    }

    // 时间戳单一维护方（§8-1）：任何更新刷新 updated_at = now
    assignments.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(Box::new(now_iso()));

    let sql = format!(
        "UPDATE devices SET {} WHERE id = ?{}",
        assignments.join(", "),
        params.len() + 1
    );
    params.push(Box::new(id));

    conn.execute(&sql, params.iter().map(|p| p.as_ref()).collect::<Vec<_>>().as_slice())?;
    get_device(conn, id)
}

/// 将 Patch<String> 日期写入参数列表：Set(非空可解析) → NaiveDate；
/// Set(空串) / Clear → NULL；Unset → 忽略。
fn push_date_assignment(
    assignments: &mut Vec<String>,
    params: &mut Vec<Box<dyn ToSql>>,
    column: &str,
    patch: &Patch<String>,
) -> Result<(), AppError> {
    match patch {
        Patch::Unset => {}
        Patch::Clear => assignments.push(format!("{} = NULL", column)),
        Patch::Set(s) if s.trim().is_empty() => assignments.push(format!("{} = NULL", column)),
        Patch::Set(s) => match parse_optional_date(&Some(s.clone())) {
            Some(date) => {
                assignments.push(format!("{} = ?{}", column, params.len() + 1));
                params.push(Box::new(date));
            }
            None => {
                return Err(AppError::validation(&format!(
                    "{} 日期格式无效: {}（支持 YYYY-MM-DD / YYYY/MM/DD / YYYY.MM.DD）",
                    column, s
                )));
            }
        },
    }
    Ok(())
}

pub fn find_device_by_serial(conn: &Connection, serial_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE serial_no = ?1 AND serial_no != '' AND {}",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![serial_no], row_to_device)?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_asset(conn: &Connection, asset_no: &str) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE asset_no = ?1 AND asset_no != '' AND {}",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![asset_no], row_to_device)?;
    Ok(rows.next().transpose()?)
}

pub fn find_device_by_name_in_rack(conn: &Connection, name: &str, rack_id: Option<i32>) -> Result<Option<Device>, AppError> {
    // 读路径默认排除软删记录（§8-13 / 团队裁决 #8）；仅用于导入查重，不得命中回收站内设备。
    match rack_id {
        Some(rid) => {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE name = ?1 AND rack_id = ?2 AND {}",
                DEVICE_SELECT, NOT_DELETED
            ))?;
            Ok(stmt.query_row(params![name, rid], row_to_device).optional()?)
        }
        None => {
            let mut stmt = conn.prepare(&format!(
                "{} WHERE name = ?1 AND rack_id IS NULL AND {}",
                DEVICE_SELECT, NOT_DELETED
            ))?;
            Ok(stmt.query_row(params![name], row_to_device).optional()?)
        }
    }
}

/// 单条软删除（N-09）。`delete_device` 的语义实现；批量删除亦复用它（§8-13）。
///
/// `UPDATE devices SET deleted_at = now, updated_at = now WHERE id = ? AND deleted_at IS NULL`，
/// 返回受影响行数是否 > 0。
pub fn soft_delete_device(conn: &Connection, id: i32) -> Result<bool, AppError> {
    let now = now_iso();
    let affected = conn.execute(
        "UPDATE devices SET deleted_at = ?1, updated_at = ?2 WHERE id = ?3 AND deleted_at IS NULL",
        params![now.as_str(), now.as_str(), id],
    )?;
    Ok(affected > 0)
}

/// 删除设备（对外语义）：按 §10.1 改为**软删除**，内部转调 `soft_delete_device`。
pub fn delete_device(conn: &Connection, id: i32) -> Result<bool, AppError> {
    soft_delete_device(conn, id)
}

/// 批量软删除（N-20）：在**同一事务**内循环复用单条软删（§8-13，禁止 `DELETE ... IN`）。
///
/// 返回 `(deleted 成功数, not_found 不存在 / 已被软删的 id 列表)`。
/// 事务的 `BEGIN/COMMIT/ROLLBACK` 由调用方（命令层）通过 `with_transaction` 包裹。
pub fn delete_devices_batch(conn: &Connection, ids: &[i32]) -> Result<(u32, Vec<i32>), AppError> {
    let mut deleted: u32 = 0;
    let mut not_found: Vec<i32> = Vec::new();
    for &id in ids {
        if soft_delete_device(conn, id)? {
            deleted += 1;
        } else {
            not_found.push(id);
        }
    }
    Ok((deleted, not_found))
}

/// 解析 ISO8601 UTC 时间戳（`%Y-%m-%dT%H:%M:%SZ`），失败返回 None。
fn parse_iso_utc(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .ok()
}

/// 冲突预检：在**其它 active 设备**中查找占用同一 `serial_no` 的记录（排除自身）。
fn find_active_by_serial_excluding(
    conn: &Connection,
    serial_no: &str,
    exclude_id: i32,
) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE serial_no = ?1 AND serial_no != '' AND {} AND id != ?2",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![serial_no, exclude_id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

/// 冲突预检：在**其它 active 设备**中查找占用同一 `asset_no` 的记录（排除自身）。
fn find_active_by_asset_excluding(
    conn: &Connection,
    asset_no: &str,
    exclude_id: i32,
) -> Result<Option<Device>, AppError> {
    let mut stmt = conn.prepare(&format!(
        "{} WHERE asset_no = ?1 AND asset_no != '' AND {} AND id != ?2",
        DEVICE_SELECT, NOT_DELETED
    ))?;
    let mut rows = stmt.query_map(params![asset_no, exclude_id], row_to_device)?;
    Ok(rows.next().transpose()?)
}

/// 恢复软删除设备（N-09）：
/// ① 读取该行 `deleted_at`，**超 30 天拒绝恢复**；
/// ② **冲突预检**——`serial_no` / `asset_no` 是否被其它 active 设备占用，命中则
///    `AppError::conflict("恢复失败：序列号 SN-x 已被设备「Y」占用，请先处理该设备")`，**不自动改名**；
/// ③ 通过则 `UPDATE devices SET deleted_at = NULL, updated_at = now WHERE id = ?` 并返回该 `Device`。
pub fn restore_device(conn: &Connection, id: i32) -> Result<Device, AppError> {
    let dev = get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))?;

    // ① 必须处于软删状态
    let deleted_at = dev
        .deleted_at
        .clone()
        .ok_or_else(|| AppError::validation("该设备未被删除，无需恢复"))?;

    // ① 超 30 天拒绝恢复（按**完整时长**判定：30 天零 1 秒即拒绝；正好 30 天放行）
    if let Some(dt) = parse_iso_utc(&deleted_at) {
        if Utc::now().signed_duration_since(dt) > chrono::Duration::days(RESTORE_WINDOW_DAYS) {
            return Err(AppError::validation(&format!(
                "恢复失败：设备删除已超过 {} 天，无法恢复",
                RESTORE_WINDOW_DAYS
            )));
        }
    }

    // ② 冲突预检（序列号）
    if !dev.serial_no.trim().is_empty() {
        if let Some(other) = find_active_by_serial_excluding(conn, &dev.serial_no, dev.id)? {
            return Err(AppError::conflict(&format!(
                "恢复失败：序列号 {} 已被设备「{}」占用，请先处理该设备",
                dev.serial_no, other.name
            )));
        }
    }
    // ② 冲突预检（资产编号）
    if !dev.asset_no.trim().is_empty() {
        if let Some(other) = find_active_by_asset_excluding(conn, &dev.asset_no, dev.id)? {
            return Err(AppError::conflict(&format!(
                "恢复失败：资产编号 {} 已被设备「{}」占用，请先处理该设备",
                dev.asset_no, other.name
            )));
        }
    }

    // ③ 恢复
    conn.execute(
        "UPDATE devices SET deleted_at = NULL, updated_at = ?1 WHERE id = ?2",
        params![now_iso(), id],
    )?;
    get_device(conn, id)?.ok_or_else(|| AppError::not_found("设备"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;
    use crate::migration;

    fn setup_db() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON").unwrap();
        migration::run(&conn).unwrap();
        conn
    }

    fn create_test_device(conn: &Connection, name: &str, rack_id: Option<i32>) -> Device {
        if let Some(rid) = rack_id {
            // 确保机柜存在（满足外键约束）
            if crate::db::racks::get_rack(conn, rid).unwrap().is_none() {
                crate::db::racks::insert_rack(conn, &crate::models::RackCreate { name: format!("TestRack{}", rid), ..Default::default() }).unwrap();
            }
        }
        insert_device(conn, &DeviceCreate {
            name: name.into(),
            rack_id,
            ..Default::default()
        }).unwrap()
    }

    #[test]
    fn test_list_empty() {
        let conn = setup_db();
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
    }

    #[test]
    fn test_create_and_list() {
        let conn = setup_db();
        create_test_device(&conn, "Web Server", None);
        let devices = list_devices(&conn, None, None).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Web Server");
    }

    #[test]
    fn test_filter_by_rack() {
        let conn = setup_db();
        create_test_device(&conn, "Dev1", Some(1));
        create_test_device(&conn, "Dev2", Some(2));
        let devices = list_devices(&conn, Some(1), None).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "Dev1");
    }

    #[test]
    fn test_search() {
        let conn = setup_db();
        create_test_device(&conn, "Core Router", None);
        create_test_device(&conn, "Access Switch", None);
        let results = list_devices(&conn, None, Some("Router".into())).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Core Router");
    }

    #[test]
    fn test_search_like_wildcard_escaped() {
        // 回归：搜索 `100%` 不得匹配名称含 100 但不含 % 的设备（LIKE 通配符转义）
        let conn = setup_db();
        create_test_device(&conn, "Rate100", None);
        create_test_device(&conn, "Rate100%Gold", None);
        // 转义后 `%100\%%` 只匹配字面 "100%"
        let results = list_devices(&conn, None, Some("100%".into())).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "Rate100%Gold");

        // `_` 同理：`A_C` 不应匹配 "ABC"
        let conn2 = setup_db();
        create_test_device(&conn2, "ABC", None);
        create_test_device(&conn2, "A_C", None);
        let results2 = list_devices(&conn2, None, Some("A_C".into())).unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].name, "A_C");
    }

    #[test]
    fn test_delete() {
        let conn = setup_db();
        create_test_device(&conn, "ToDelete", None);
        assert!(delete_device(&conn, 1).unwrap());
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
    }

    #[test]
    fn test_delete_nonexistent() {
        let conn = setup_db();
        assert!(!delete_device(&conn, 999).unwrap());
    }

    /// 上架一台设备（分配到机柜 1 的 U5）
    fn mount_device(conn: &Connection, name: &str) -> Device {
        // 确保机柜 1 存在（满足外键约束）
        if crate::db::racks::get_rack(conn, 1).unwrap().is_none() {
            crate::db::racks::insert_rack(conn, &crate::models::RackCreate {
                name: "MountRack".into(),
                ..Default::default()
            }).unwrap();
        }
        insert_device(conn, &DeviceCreate {
            name: name.into(),
            rack_id: Some(1),
            start_u: Some(5),
            end_u: Some(7),
            status: Some("offline".into()),
            ..Default::default()
        }).unwrap()
    }

    #[test]
    fn test_insert_derives_height_from_u_span() {
        // 无型号设备：插入时高度由 start_u/end_u 区间推导
        let conn = setup_db();
        let dev = mount_device(&conn, "MultiU");
        assert_eq!(dev.start_u, Some(5));
        assert_eq!(dev.end_u, Some(7));
        assert_eq!(dev.height_u, 3);
    }

    #[test]
    fn test_height_persists_after_unrack_and_rerack() {
        // 回归：P0 — 多U设备下架后重上架退化为 1U
        let conn = setup_db();
        let dev = mount_device(&conn, "MultiU");
        assert_eq!(dev.height_u, 3);

        // 1) 下架：清空机柜与 U 位，height_u 必须保留
        let unracked = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Clear,
            start_u: Patch::Clear,
            end_u: Patch::Clear,
            status: Patch::Set("unconfigured".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(unracked.rack_id, None);
        assert_eq!(unracked.height_u, 3, "下架后固有高度不得丢失");

        // 2) 重上架到新位置（只传机柜与 U 位，模拟拖拽）
        let racked = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Set(1),
            start_u: Patch::Set(2),
            end_u: Patch::Set(4),
            status: Patch::Set("offline".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(racked.start_u, Some(2));
        assert_eq!(racked.end_u, Some(4));
        assert_eq!(racked.height_u, 3, "重上架后仍应为 3U");
    }

    #[test]
    fn test_update_clear_racks_off_shelf() {
        // 回归：P0 — COALESCE 无法置空，导致"拖回资源池/未上架"失效
        let conn = setup_db();
        let dev = mount_device(&conn, "ToUnmount");
        assert_eq!(dev.rack_id, Some(1));

        // 模拟 RackView.handleStockDrop / DeviceDetailPanel「未上架」：显式传 null
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            rack_id: Patch::Clear,
            start_u: Patch::Clear,
            end_u: Patch::Clear,
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.rack_id, None);
        assert_eq!(updated.start_u, None);
        assert_eq!(updated.end_u, None);
        assert_eq!(updated.status, "offline"); // 其它字段不受影响
    }

    #[test]
    fn test_update_partial_keeps_other_columns() {
        let conn = setup_db();
        let dev = mount_device(&conn, "PartialUpdate");
        // 只更新 status（在线），rack/u 位必须保持不变
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            status: Patch::Set("online".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.status, "online");
        assert_eq!(updated.rack_id, Some(1));
        assert_eq!(updated.start_u, Some(5));
        assert_eq!(updated.end_u, Some(7));
    }

    #[test]
    fn test_update_set_field_values() {
        let conn = setup_db();
        let dev = mount_device(&conn, "Rename");
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            name: Patch::Set("Renamed".into()),
            serial_no: Patch::Set("SN-001".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.name, "Renamed");
        assert_eq!(updated.serial_no, "SN-001");
        assert_eq!(updated.rack_id, Some(1)); // 未提及字段保持
    }

    #[test]
    fn test_update_clear_text_and_date() {
        let conn = setup_db();
        let dev = mount_device(&conn, "ClearFields");
        // 文本置空字符串 + 日期 Clear → NULL
        let updated = update_device(&conn, dev.id, &DeviceUpdate {
            serial_no: Patch::Set("".into()),
            purchase_date: Patch::Clear,
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(updated.serial_no, "");
        assert_eq!(updated.purchase_date, None);
    }

    #[test]
    fn test_update_unknown_id_returns_none() {
        let conn = setup_db();
        let result = update_device(&conn, 999, &DeviceUpdate {
            name: Patch::Set("X".into()),
            ..Default::default()
        }).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_update_date_invalid_format() {
        let conn = setup_db();
        let dev = mount_device(&conn, "BadDate");
        let result = update_device(&conn, dev.id, &DeviceUpdate {
            purchase_date: Patch::Set("2026-13-99".into()),
            ..Default::default()
        });
        assert!(result.is_err());
    }

    // ==================== N-01/N-02 query_devices ====================

    #[test]
    fn test_query_devices_excludes_deleted_by_default() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Alive", None);
        let gone = create_test_device(&conn, "Gone", None);
        soft_delete_device(&conn, gone.id).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery::default()).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, dev.id);

        // include_deleted = true 时全部返回
        let (items2, total2) = query_devices(&conn, &DeviceQuery {
            include_deleted: Some(true),
            ..Default::default()
        }).unwrap();
        assert_eq!(total2, 2);
        assert_eq!(items2.len(), 2);
    }

    #[test]
    fn test_query_devices_multi_field_search() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "TOK-Name".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "B".into(), ip_addresses: Some("10.99.0.1".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "C".into(), serial_no: Some("TOK-SN".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "D".into(), asset_no: Some("TOK-AS".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Nope".into(), ..Default::default() }).unwrap();

        // name / serial_no / asset_no 三字段命中（N-10 后 IP 需为合法格式，IP 命中单独断言）
        let (items, total) = query_devices(&conn, &DeviceQuery {
            search: Some("TOK".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 3);
        assert_eq!(items.len(), 3);

        // ip_addresses 字段命中
        let (items_ip, total_ip) = query_devices(&conn, &DeviceQuery {
            search: Some("10.99".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total_ip, 1);
        assert_eq!(items_ip.len(), 1);
        assert_eq!(items_ip[0].name, "B");
    }

    // ==================== B1 全局搜索（五字段含 owner） ====================

    /// B1：`query_devices` 搜索命中 owner（责任人）字段
    #[test]
    fn test_query_devices_owner_search() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "Web-01".into(), owner: Some("张三".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Web-02".into(), owner: Some("李四".into()), ..Default::default() }).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery {
            search: Some("张三".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 1);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "Web-01");
    }

    /// B1：`list_devices`（机柜视图搜索）与 `query_devices` 搜索口径一致——
    /// 同为五字段（name / ip_addresses / serial_no / asset_no / owner）。
    #[test]
    fn test_list_devices_multi_field_search_including_owner() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "A".into(), owner: Some("王五".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "B".into(), ip_addresses: Some("10.0.0.8".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "C".into(), serial_no: Some("SN-LIST".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "D".into(), asset_no: Some("AS-LIST".into()), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Nope".into(), ..Default::default() }).unwrap();

        // owner 命中
        let hit_owner = list_devices(&conn, None, Some("王五".into())).unwrap();
        assert_eq!(hit_owner.len(), 1);
        assert_eq!(hit_owner[0].name, "A");
        // IP / SN / 资产编号命中
        assert_eq!(list_devices(&conn, None, Some("10.0.0".into())).unwrap().len(), 1);
        assert_eq!(list_devices(&conn, None, Some("SN-LIST".into())).unwrap().len(), 1);
        assert_eq!(list_devices(&conn, None, Some("AS-LIST".into())).unwrap().len(), 1);
        // 无关词不命中
        assert!(list_devices(&conn, None, Some("NoMatch".into())).unwrap().is_empty());
    }

    #[test]
    fn test_query_devices_sort_whitelist() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "Zeta".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Alpha".into(), ..Default::default() }).unwrap();

        let (items, total) = query_devices(&conn, &DeviceQuery {
            sort_field: Some("name".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 2);
        assert_eq!(items[0].name, "Zeta");
        assert_eq!(items[1].name, "Alpha");

        // 非白名单字段回退 name（不得报错、不得注入）
        let (items_fallback, _) = query_devices(&conn, &DeviceQuery {
            sort_field: Some("name; DROP TABLE devices".into()),
            sort_order: Some("desc".into()),
            ..Default::default()
        }).unwrap();
        assert_eq!(items_fallback.len(), 2);
        assert_eq!(items_fallback[0].name, "Zeta");
    }

    #[test]
    fn test_query_devices_pagination_and_filter() {
        let conn = setup_db();
        for i in 1..=5 {
            create_test_device(&conn, &format!("Dev{}", i), Some(1));
        }
        create_test_device(&conn, "Other", Some(2));

        // 机柜过滤 + 分页
        let (page1, total) = query_devices(&conn, &DeviceQuery {
            rack_id: Some(1),
            limit: Some(2),
            offset: Some(0),
            ..Default::default()
        }).unwrap();
        assert_eq!(total, 5);
        assert_eq!(page1.len(), 2);

        let (page2, _) = query_devices(&conn, &DeviceQuery {
            rack_id: Some(1),
            limit: Some(2),
            offset: Some(2),
            ..Default::default()
        }).unwrap();
        assert_eq!(page2.len(), 2);
        // 两页不重叠
        assert_ne!(page1[0].id, page2[0].id);
    }

    // ==================== N-09 soft delete / restore ====================

    #[test]
    fn test_soft_delete_hides_and_supports_recreate_same_serial() {
        let conn = setup_db();
        let first = insert_device(&conn, &DeviceCreate {
            name: "Old".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        }).unwrap();

        assert!(soft_delete_device(&conn, first.id).unwrap());
        // 默认不可见
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
        // 回收站可见
        let deleted = list_deleted_devices(&conn, None).unwrap();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].id, first.id);

        // 同序列号可重建（部分唯一索引生效）
        let second = insert_device(&conn, &DeviceCreate {
            name: "New".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        }).unwrap();
        assert_ne!(first.id, second.id);

        // active 之间仍强唯一
        let dup = insert_device(&conn, &DeviceCreate {
            name: "Dup".into(),
            serial_no: Some("SN-X".into()),
            ..Default::default()
        });
        assert!(dup.is_err(), "active 重复序列号必须被拒");
    }

    #[test]
    fn test_soft_delete_device_idempotent_returns_false() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Once", None);
        assert!(soft_delete_device(&conn, dev.id).unwrap());
        // 再次软删：已删（deleted_at 非 NULL）→ 0 行
        assert!(!soft_delete_device(&conn, dev.id).unwrap());
        // 不存在的 id
        assert!(!soft_delete_device(&conn, 9999).unwrap());
    }

    #[test]
    fn test_restore_device_success() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "RestoreMe", None);
        soft_delete_device(&conn, dev.id).unwrap();
        let restored = restore_device(&conn, dev.id).unwrap();
        assert_eq!(restored.id, dev.id);
        assert!(restored.deleted_at.is_none());
        assert_eq!(list_devices(&conn, None, None).unwrap().len(), 1);
    }

    #[test]
    fn test_restore_conflict_returns_conflict() {
        use crate::error::ErrorCode;
        let conn = setup_db();
        let old = insert_device(&conn, &DeviceCreate {
            name: "Old".into(),
            serial_no: Some("SN-C".into()),
            ..Default::default()
        }).unwrap();
        soft_delete_device(&conn, old.id).unwrap();

        // 软删期间用同序列号建了新设备
        insert_device(&conn, &DeviceCreate {
            name: "New".into(),
            serial_no: Some("SN-C".into()),
            ..Default::default()
        }).unwrap();

        let err = restore_device(&conn, old.id).unwrap_err();
        match err {
            AppError { code: ErrorCode::Conflict, .. } => {}
            other => panic!("期望 Conflict，实际: {:?}", other),
        }
    }

    #[test]
    fn test_restore_over_30_days_rejected() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "TooOld", None);
        soft_delete_device(&conn, dev.id).unwrap();

        // 将 deleted_at 回拨到 31 天前
        let past = (chrono::Utc::now() - chrono::Duration::days(31))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        conn.execute(
            "UPDATE devices SET deleted_at = ?1 WHERE id = ?2",
            params![past, dev.id],
        ).unwrap();

        let err = restore_device(&conn, dev.id);
        assert!(err.is_err(), "超过 30 天必须拒绝恢复");
    }

    #[test]
    fn test_restore_active_device_rejected() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Active", None);
        // 未软删直接恢复 → 拒绝
        assert!(restore_device(&conn, dev.id).is_err());
    }

    // ==================== N-20 delete_devices_batch ====================

    #[test]
    fn test_delete_devices_batch_counts_and_not_found() {
        let conn = setup_db();
        let a = create_test_device(&conn, "A", None);
        let b = create_test_device(&conn, "B", None);

        let (deleted, not_found) = delete_devices_batch(&conn, &[a.id, b.id, 9999]).unwrap();
        assert_eq!(deleted, 2);
        assert_eq!(not_found, vec![9999]);
        assert!(list_devices(&conn, None, None).unwrap().is_empty());
        assert_eq!(list_deleted_devices(&conn, None).unwrap().len(), 2);
    }

    #[test]
    fn test_delete_devices_batch_already_deleted_goes_to_not_found() {
        let conn = setup_db();
        let a = create_test_device(&conn, "A", None);
        soft_delete_device(&conn, a.id).unwrap();

        let (deleted, not_found) = delete_devices_batch(&conn, &[a.id]).unwrap();
        assert_eq!(deleted, 0);
        assert_eq!(not_found, vec![a.id]);
    }

    // ==================== Stage 2A 对抗性验证（QA 补充） ====================

    /// 点 2②：`query_devices` 搜索对 `%` / `_` / `\` 三个 LIKE 通配符**全部**转义，
    /// 含这些字符的设备名不会误命中「不含该字面字符」的记录。
    #[test]
    fn test_query_devices_escapes_all_like_wildcards() {
        // 字面 `%`
        let conn = setup_db();
        let p = insert_device(&conn, &DeviceCreate { name: "HasPercent%Here".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "HasPercentXHere".into(), ..Default::default() }).unwrap();
        let (items, total) = query_devices(&conn, &DeviceQuery { search: Some("%".into()), ..Default::default() }).unwrap();
        assert_eq!(total, 1, "搜索 % 只应命中含字面 % 的记录（不得当通配符）");
        assert_eq!(items[0].id, p.id);

        // 字面 `_`
        let conn2 = setup_db();
        let u = insert_device(&conn2, &DeviceCreate { name: "A_C".into(), ..Default::default() }).unwrap();
        insert_device(&conn2, &DeviceCreate { name: "ABC".into(), ..Default::default() }).unwrap();
        let (items2, total2) = query_devices(&conn2, &DeviceQuery { search: Some("_".into()), ..Default::default() }).unwrap();
        assert_eq!(total2, 1, "搜索 _ 只应命中含字面 _ 的记录");
        assert_eq!(items2[0].id, u.id);

        // 字面 `\`（转义顺序若写反会在此暴露）
        let conn3 = setup_db();
        let b = insert_device(&conn3, &DeviceCreate { name: "back\\slash".into(), ..Default::default() }).unwrap();
        insert_device(&conn3, &DeviceCreate { name: "backslash".into(), ..Default::default() }).unwrap();
        let (items3, total3) = query_devices(&conn3, &DeviceQuery { search: Some("\\".into()), ..Default::default() }).unwrap();
        assert_eq!(total3, 1, "搜索 \\ 只应命中含字面反斜杠的记录");
        assert_eq!(items3[0].id, b.id);
    }

    /// 点 2①：`sort_order` 同样仅接受白名单；非法值安全降级为 ASC，且不破坏表结构。
    #[test]
    fn test_query_devices_sort_order_injection_safe() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "Zeta".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Alpha".into(), ..Default::default() }).unwrap();

        let (items, _) = query_devices(&conn, &DeviceQuery {
            sort_field: Some("name".into()),
            sort_order: Some("desc; DROP TABLE devices".into()),
            ..Default::default()
        }).unwrap();
        // 非法 sort_order → 降级 ASC
        assert_eq!(items[0].name, "Alpha");
        assert_eq!(items[1].name, "Zeta");
        // 表仍完好（注入未生效）
        assert_eq!(list_devices(&conn, None, None).unwrap().len(), 2);
    }

    /// 点 2③：`include_deleted=true` 只在结果中多出软删记录，不外泄其它状态；
    /// 返回集合恰好 = {active} ∪ {deleted}。
    #[test]
    fn test_query_devices_include_deleted_only_adds_deleted() {
        let conn = setup_db();
        let a = create_test_device(&conn, "A", None);
        let b = create_test_device(&conn, "B", None);
        let d = create_test_device(&conn, "D", None);
        soft_delete_device(&conn, d.id).unwrap();

        let (active, t0) = query_devices(&conn, &DeviceQuery::default()).unwrap();
        assert_eq!(t0, 2);
        let mut active_ids: Vec<i32> = active.iter().map(|x| x.id).collect();
        active_ids.sort();
        assert_eq!(active_ids, vec![a.id, b.id]);
        assert!(active.iter().all(|x| x.deleted_at.is_none()), "默认查询不得返回任何已删记录");

        let (all, t1) = query_devices(&conn, &DeviceQuery { include_deleted: Some(true), ..Default::default() }).unwrap();
        assert_eq!(t1, 3);
        let mut all_ids: Vec<i32> = all.iter().map(|x| x.id).collect();
        all_ids.sort();
        assert_eq!(all_ids, vec![a.id, b.id, d.id]);
        // 恰好一条处于软删状态：既没漏记录，也没把 active 错标为删除
        assert_eq!(all.iter().filter(|x| x.deleted_at.is_some()).count(), 1);
    }

    /// 点 2④：`total` 与 `items` 口径一致 —— 同一 WHERE，`total` 忽略 LIMIT/OFFSET。
    #[test]
    fn test_query_devices_total_consistent_with_items_where() {
        let conn = setup_db();
        insert_device(&conn, &DeviceCreate { name: "Multi-1".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Multi-2".into(), ..Default::default() }).unwrap();
        insert_device(&conn, &DeviceCreate { name: "Multi-3".into(), ..Default::default() }).unwrap();

        // 搜索命中 3 条，但只取 1 条：total 仍须为 3
        let (items, total) = query_devices(&conn, &DeviceQuery {
            search: Some("Multi".into()),
            limit: Some(1),
            offset: Some(0),
            ..Default::default()
        }).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(total, 3, "total 应与 items 同 WHERE，不受 LIMIT 影响");
    }

    /// 点 3①：恢复窗口边界 —— 窗口内（30 天减 5 秒）应可恢复；31 天拒绝。
    ///
    /// 注：秒级时间戳下「正好 30 天」**不可构造** —— `%Y-%m-%dT%H:%M:%SZ` 向下截断会使存量值
    /// 比「30 天前」更早，叠加 `restore_device` 内取的更晚 `now`，`age` 必然 > 30 天。
    /// 故 allowed 侧用**明确落在窗口内**的「30 天减 5 秒」代表（5 秒余量足以覆盖截断 ≤1 秒
    /// 与测试执行间隔）；拒绝侧由 `test_restore_boundary_just_over_30_days_should_reject`
    /// 与 `test_restore_boundary_31_days_rejected` 覆盖。
    #[test]
    fn test_restore_boundary_within_30_days_allowed() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Within30", None);
        soft_delete_device(&conn, dev.id).unwrap();
        let past = (chrono::Utc::now() - chrono::Duration::days(30) + chrono::Duration::seconds(5))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        conn.execute("UPDATE devices SET deleted_at = ?1 WHERE id = ?2", params![past, dev.id]).unwrap();
        let restored = restore_device(&conn, dev.id).expect("30 天减 5 秒仍在窗口内，应可恢复");
        assert!(restored.deleted_at.is_none());
    }

    #[test]
    fn test_restore_boundary_31_days_rejected() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "Day31", None);
        soft_delete_device(&conn, dev.id).unwrap();
        let past = (chrono::Utc::now() - chrono::Duration::days(31))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        conn.execute("UPDATE devices SET deleted_at = ?1 WHERE id = ?2", params![past, dev.id]).unwrap();
        assert!(restore_device(&conn, dev.id).is_err(), "超过 30 天必须拒绝恢复");
    }

    /// 点 3①（边界）：删除时间 = 30 天 + 1 小时 → 严格超出 30 天窗口，必须拒绝。
    ///
    /// 设计口径为「超过 30 天拒绝恢复」（§4.3 / §9 Q6）。`restore_device` 已改为
    /// 按**完整时长**判定：`Utc::now().signed_duration_since(deleted_at) >
    /// chrono::Duration::days(30)`。因此「30 天零 1 秒」即拒绝，而**正好 30 天**放行
    /// （见 `test_restore_boundary_exactly_30_days_allowed`）。
    #[test]
    fn test_restore_boundary_just_over_30_days_should_reject() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "OverByHour", None);
        soft_delete_device(&conn, dev.id).unwrap();
        let past = (chrono::Utc::now() - chrono::Duration::days(30) - chrono::Duration::hours(1))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();
        conn.execute("UPDATE devices SET deleted_at = ?1 WHERE id = ?2", params![past, dev.id]).unwrap();
        assert!(
            restore_device(&conn, dev.id).is_err(),
            "删除已超过 30 天窗口，按设计应拒绝恢复"
        );
    }

    /// 点 3②：恢复冲突预检必须覆盖**第二个**唯一键（asset_no），不能只查 serial_no。
    #[test]
    fn test_restore_conflict_on_asset_no_rejected() {
        use crate::error::ErrorCode;
        let conn = setup_db();
        let old = insert_device(&conn, &DeviceCreate {
            name: "OldAsset".into(),
            asset_no: Some("AS-C".into()),
            ..Default::default()
        }).unwrap();
        soft_delete_device(&conn, old.id).unwrap();

        // 软删期间用同资产编号建了新设备（serial 不同，仅 asset 冲突）
        insert_device(&conn, &DeviceCreate {
            name: "NewAsset".into(),
            asset_no: Some("AS-C".into()),
            ..Default::default()
        }).unwrap();

        let err = restore_device(&conn, old.id).unwrap_err();
        match err {
            AppError { code: ErrorCode::Conflict, .. } => {}
            other => panic!("期望 asset_no 冲突返回 Conflict，实际: {:?}", other),
        }
    }

    /// 点 3③：恢复成功后 `deleted_at` 置空、`updated_at` 刷新为当前时刻。
    #[test]
    fn test_restore_refreshes_updated_at_and_clears_deleted_at() {
        let conn = setup_db();
        let dev = create_test_device(&conn, "RestoreStamp", None);
        soft_delete_device(&conn, dev.id).unwrap();
        // deleted_at 保持「刚删」（在窗口内）；仅把 updated_at 拨到历史值，观察是否刷新
        conn.execute(
            "UPDATE devices SET updated_at = '2000-01-01T00:00:00Z' WHERE id = ?1",
            params![dev.id],
        ).unwrap();

        let restored = restore_device(&conn, dev.id).unwrap();
        assert!(restored.deleted_at.is_none(), "恢复后 deleted_at 必须置空");
        assert_ne!(
            restored.updated_at.as_deref(),
            Some("2000-01-01T00:00:00Z"),
            "恢复后 updated_at 必须刷新为当前时刻"
        );
        assert!(restored.updated_at.is_some());
    }

    /// 点 4③（关键）：批量软删在**同一事务**内执行；任一环节失败 → 整体 ROLLBACK，不留部分删除。
    /// 构造：同一事务内先批量软删 2 台，再触发唯一约束冲突使事务失败。
    #[test]
    fn test_delete_devices_batch_atomic_rollback() {
        let conn = setup_db();
        let a = create_test_device(&conn, "AtomicA", None);
        let b = create_test_device(&conn, "AtomicB", None);
        // 占位 active 设备，持有 SN-DUP
        insert_device(&conn, &DeviceCreate {
            name: "Holder".into(),
            serial_no: Some("SN-DUP".into()),
            ..Default::default()
        }).unwrap();

        let outcome: Result<(), AppError> = crate::db::with_transaction(&conn, |c| {
            let (deleted, not_found) = delete_devices_batch(c, &[a.id, b.id])?;
            assert_eq!(deleted, 2);
            assert!(not_found.is_empty());
            // 同一事务内制造失败：插入重复序列号 → 部分唯一索引拒绝
            let _ = insert_device(c, &DeviceCreate {
                name: "Dup".into(),
                serial_no: Some("SN-DUP".into()),
                ..Default::default()
            })?;
            Ok(())
        });
        assert!(outcome.is_err(), "事务内冲突应使整体失败");

        // 原子性：a、b 均不得被软删（不留部分删除）
        assert_eq!(
            list_devices(&conn, None, None).unwrap().len(),
            3,
            "回滚后 3 台在用设备均应保留"
        );
        assert!(
            list_deleted_devices(&conn, None).unwrap().is_empty(),
            "回滚后回收站必须为空（不得留下部分删除）"
        );
    }

    // ===================== N-10 输入验证 =====================

    /// 建 10U 机柜并返回其 id
    fn create_test_rack_10u(conn: &Connection, name: &str) -> i32 {
        crate::db::racks::insert_rack(conn, &crate::models::RackCreate {
            name: name.into(),
            height_u: Some(10),
            ..Default::default()
        })
        .unwrap()
        .id
    }

    fn assert_validation(err: AppError, hint: &str) {
        assert!(
            err.message.contains(hint),
            "错误消息应含「{hint}」，实际：{}",
            err.message
        );
    }

    #[test]
    fn test_n10_u_range_rejected() {
        let conn = setup_db();
        // start > end
        let err = insert_device(&conn, &DeviceCreate {
            name: "Bad-Range".into(),
            start_u: Some(5),
            end_u: Some(3),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "结束U位(3)不能小于起始U位(5)");
        // U 位 < 1
        let err = insert_device(&conn, &DeviceCreate {
            name: "Bad-Zero".into(),
            start_u: Some(0),
            end_u: Some(2),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "U 位(0)无效");
        // update 路径同样拦截
        let dev = insert_device(&conn, &DeviceCreate { name: "OK".into(), ..Default::default() }).unwrap();
        let err = update_device(&conn, dev.id, &DeviceUpdate {
            start_u: Patch::Set(9),
            end_u: Patch::Set(4),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "结束U位(4)不能小于起始U位(9)");
    }

    #[test]
    fn test_n10_u_boundary_rejected() {
        let conn = setup_db();
        let rid = create_test_rack_10u(&conn, "R42");
        // end 超出机柜高度
        let err = insert_device(&conn, &DeviceCreate {
            name: "Too-Tall".into(),
            rack_id: Some(rid),
            start_u: Some(9),
            end_u: Some(11),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "超出机柜「R42」高度(10U)");
        // 边界内合法
        insert_device(&conn, &DeviceCreate {
            name: "Edge-OK".into(),
            rack_id: Some(rid),
            start_u: Some(9),
            end_u: Some(10),
            ..Default::default()
        }).unwrap();
    }

    #[test]
    fn test_n10_u_overlap_insert_rejected() {
        let conn = setup_db();
        let rid = create_test_rack_10u(&conn, "R10");
        insert_device(&conn, &DeviceCreate {
            name: "A".into(), rack_id: Some(rid),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
        // 与 A(1-4) 相交：区间包含/部分重叠均拒绝
        for (s, e) in [(3, 6), (4, 4), (1, 10)] {
            let err = insert_device(&conn, &DeviceCreate {
                name: format!("B-{s}-{e}"),
                rack_id: Some(rid),
                start_u: Some(s),
                end_u: Some(e),
                ..Default::default()
            }).unwrap_err();
            assert_validation(err, "重叠");
        }
        // 相邻（5-8 紧接 1-4）合法
        insert_device(&conn, &DeviceCreate {
            name: "Adjacent".into(), rack_id: Some(rid),
            start_u: Some(5), end_u: Some(8), ..Default::default()
        }).unwrap();
        // 不同机柜同 U 位合法
        let rid2 = create_test_rack_10u(&conn, "R10-B");
        insert_device(&conn, &DeviceCreate {
            name: "OtherRack".into(), rack_id: Some(rid2),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
    }

    #[test]
    fn test_n10_u_overlap_update_excludes_self() {
        let conn = setup_db();
        let rid = create_test_rack_10u(&conn, "R10");
        let a = insert_device(&conn, &DeviceCreate {
            name: "A".into(), rack_id: Some(rid),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
        // 自身移动（新区间与旧区间相交）不构成自重叠 → 成功
        let moved = update_device(&conn, a.id, &DeviceUpdate {
            start_u: Patch::Set(2),
            end_u: Patch::Set(5),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!((moved.start_u, moved.end_u), (Some(2), Some(5)));
        // 高度自动同步为区间高度
        assert_eq!(moved.height_u, 4);
        // 再移到与"其它设备"重叠处 → conflict（移 end 到 7：eff 2-7 与 B(7-8) 相交）
        insert_device(&conn, &DeviceCreate {
            name: "B".into(), rack_id: Some(rid),
            start_u: Some(7), end_u: Some(8), ..Default::default()
        }).unwrap();
        let err = update_device(&conn, a.id, &DeviceUpdate {
            end_u: Patch::Set(7),
            ..Default::default()
        }).unwrap_err();
        assert!(err.message.contains("重叠"), "应报重叠，实际：{}", err.message);
    }

    #[test]
    fn test_n10_u_overlap_soft_deleted_ignored() {
        let conn = setup_db();
        let rid = create_test_rack_10u(&conn, "R10");
        let a = insert_device(&conn, &DeviceCreate {
            name: "A".into(), rack_id: Some(rid),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
        delete_device(&conn, a.id).unwrap();
        // A 软删后其 U 位可复用（v5 回收站语义：软删退出占用）
        insert_device(&conn, &DeviceCreate {
            name: "Reuse".into(), rack_id: Some(rid),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
    }

    #[test]
    fn test_n10_update_non_space_fields_not_blocked_by_legacy_overlap() {
        // 历史脏数据（直接 SQL 造重叠）编辑名称/状态等非空间字段不受阻；
        // 只有动 U 位时才要求解决冲突
        let conn = setup_db();
        let rid = create_test_rack_10u(&conn, "R10");
        insert_device(&conn, &DeviceCreate {
            name: "A".into(), rack_id: Some(rid),
            start_u: Some(1), end_u: Some(4), ..Default::default()
        }).unwrap();
        // 直接 SQL 构造历史脏数据（绕过 N-10 校验，模拟校验上线前的存量重叠）
        conn.execute(
            "INSERT INTO devices (name, rack_id, start_u, end_u) VALUES ('B', ?1, 2, 3)",
            params![rid],
        ).unwrap();
        let b_id: i32 = conn.last_insert_rowid() as i32;
        // 非空间编辑 → 成功
        let renamed = update_device(&conn, b_id, &DeviceUpdate {
            name: Patch::Set("B-Renamed".into()),
            status: Patch::Set("online".into()),
            ..Default::default()
        }).unwrap().unwrap();
        assert_eq!(renamed.name, "B-Renamed");
        // 动 U 位 → 被 overlap 拦截
        let err = update_device(&conn, b_id, &DeviceUpdate {
            end_u: Patch::Set(5),
            ..Default::default()
        }).unwrap_err();
        assert!(err.message.contains("重叠"), "应报重叠，实际：{}", err.message);
    }

    #[test]
    fn test_n10_ip_addresses() {
        let conn = setup_db();
        // 合法：IPv4 / 多 IP 混合分隔符 / IPv6 / 空白串
        for ip in ["10.0.0.1", "10.0.0.1,10.0.0.2", "10.0.0.1； 192.168.1.1", "::1", "2001:db8::1", "   "] {
            insert_device(&conn, &DeviceCreate {
                name: format!("IP-OK-{ip}"),
                ip_addresses: Some(ip.into()),
                ..Default::default()
            }).unwrap();
        }
        // 非法：越界段 / 非数字 / 前导零 / 段数不对
        for ip in ["256.1.1.1", "1.2.3.4.5", "abc.def.ghi.jkl", "01.2.3.4", "1.2.3"] {
            let err = insert_device(&conn, &DeviceCreate {
                name: format!("IP-Bad-{ip}"),
                ip_addresses: Some(ip.into()),
                ..Default::default()
            }).unwrap_err();
            assert_validation(err, "IP 地址格式不正确");
        }
        // update：Set 新值校验；Clear（置 NULL）放行
        let dev = insert_device(&conn, &DeviceCreate {
            name: "Upd".into(),
            ip_addresses: Some("10.0.0.9".into()),
            ..Default::default()
        }).unwrap();
        let err = update_device(&conn, dev.id, &DeviceUpdate {
            ip_addresses: Patch::Set("999.0.0.1".into()),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "IP 地址格式不正确");
        update_device(&conn, dev.id, &DeviceUpdate {
            ip_addresses: Patch::Clear,
            ..Default::default()
        }).unwrap().unwrap();
    }

    #[test]
    fn test_n10_status_enum() {
        let conn = setup_db();
        // 合法：白名单三值（None → 默认 unconfigured）
        for st in ["online", "offline", "unconfigured"] {
            insert_device(&conn, &DeviceCreate {
                name: format!("ST-{st}"),
                status: Some(st.into()),
                ..Default::default()
            }).unwrap();
        }
        // 非法：未知值 / 空串
        for st in ["running", ""] {
            let err = insert_device(&conn, &DeviceCreate {
                name: format!("ST-Bad-{st}"),
                status: Some(st.into()),
                ..Default::default()
            }).unwrap_err();
            assert_validation(err, "设备状态取值非法");
        }
        // update：Set 非法值拦截
        let dev = insert_device(&conn, &DeviceCreate { name: "Upd-ST".into(), ..Default::default() }).unwrap();
        let err = update_device(&conn, dev.id, &DeviceUpdate {
            status: Patch::Set("maintenance".into()),
            ..Default::default()
        }).unwrap_err();
        assert_validation(err, "设备状态取值非法");
    }
}
