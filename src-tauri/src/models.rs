use serde::{Deserialize, Serialize};
use chrono::NaiveDate;
use std::fmt;

/// 三态更新字段（用于 Update 结构体，替代无法置空的 COALESCE 部分更新）：
/// - `Unset`：字段未在请求中提供 —— 不更新（保持原值）
/// - `Set(v)`：字段显式提供 —— 更新为 v
/// - `Clear`：字段显式传 null —— 置为 NULL（下架、解除关联等）
///
/// 序列化约定：JSON `null` → `Clear`；JSON 值 → `Set(v)`；
/// 字段缺失（配合 `#[serde(default)]`）→ `Unset`。
#[derive(Debug, Clone, Default, PartialEq)]
pub enum Patch<T> {
    #[default]
    Unset,
    Set(T),
    Clear,
}

impl<T> fmt::Display for Patch<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Patch::Unset => write!(f, "Unset"),
            Patch::Set(_) => write!(f, "Set"),
            Patch::Clear => write!(f, "Clear"),
        }
    }
}

impl<T: Serialize> Serialize for Patch<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Patch::Set(v) => v.serialize(serializer),
            Patch::Unset | Patch::Clear => serializer.serialize_none(),
        }
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Patch<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // null → Clear；其它值解析为 T → Set
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(v) => Patch::Set(v),
            None => Patch::Clear,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceModel {
    pub id: i32,
    pub name: String,
    pub manufacturer: String,
    #[serde(rename = "type")]
    pub device_type: String,
    pub height_u: i32,
    pub power_watt: i32,
    /// 创建时间（UTC ISO8601，Rust 内部维护，下同）
    pub created_at: Option<String>,
    /// 最近更新时间
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Room {
    pub id: i32,
    pub name: String,
    pub location: String,
    pub sort_order: i32,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Rack {
    pub id: i32,
    pub name: String,
    pub height_u: i32,
    pub row: i32,
    pub col: i32,
    pub view: String,
    pub sort_order: i32,
    pub room_id: Option<i32>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Device {
    pub id: i32,
    pub name: String,
    pub device_model_id: Option<i32>,
    pub rack_id: Option<i32>,
    pub start_u: Option<i32>,
    pub end_u: Option<i32>,
    pub ip_addresses: String,
    pub serial_no: String,
    pub asset_no: String,
    pub department: String,
    pub owner: String,
    pub function: String,
    pub purchase_date: Option<NaiveDate>,
    pub warranty_expire: Option<NaiveDate>,
    pub status: String,
    pub power_watt: i32,
    /// 设备固有高度（U）。v4 起持久化：下架/重上架不再丢失（回归：多U设备重上架变 1U）。
    pub height_u: i32,
    /// 创建时间（UTC ISO8601，Rust 内部维护）
    pub created_at: Option<String>,
    /// 最近更新时间（insert 时等于 created_at；update/软删时刷新）
    pub updated_at: Option<String>,
    /// 软删除标记（N-09）：NULL = 未删除；非 NULL = 回收站中。
    /// 除回收站 / `include_deleted=true` 外，所有查询必须过滤 `deleted_at IS NULL`。
    pub deleted_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceModelCreate {
    pub name: String,
    pub manufacturer: Option<String>,
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    pub height_u: Option<i32>,
    pub power_watt: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceModelUpdate {
    #[serde(default)] pub name: Patch<String>,
    #[serde(default)] pub manufacturer: Patch<String>,
    #[serde(default, rename = "type")] pub device_type: Patch<String>,
    #[serde(default)] pub height_u: Patch<i32>,
    #[serde(default)] pub power_watt: Patch<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RoomCreate {
    pub name: String,
    pub location: Option<String>,
    pub sort_order: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RoomUpdate {
    #[serde(default)] pub name: Patch<String>,
    #[serde(default)] pub location: Patch<String>,
    #[serde(default)] pub sort_order: Patch<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RackCreate {
    pub name: String,
    pub height_u: Option<i32>,
    pub row: Option<i32>,
    pub col: Option<i32>,
    pub view: Option<String>,
    pub sort_order: Option<i32>,
    pub room_id: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct RackUpdate {
    #[serde(default)] pub name: Patch<String>,
    #[serde(default)] pub height_u: Patch<i32>,
    #[serde(default)] pub row: Patch<i32>,
    #[serde(default)] pub col: Patch<i32>,
    #[serde(default)] pub view: Patch<String>,
    #[serde(default)] pub sort_order: Patch<i32>,
    #[serde(default)] pub room_id: Patch<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceCreate {
    pub name: String,
    pub device_model_id: Option<i32>,
    pub rack_id: Option<i32>,
    pub start_u: Option<i32>,
    pub end_u: Option<i32>,
    pub ip_addresses: Option<String>,
    pub serial_no: Option<String>,
    pub asset_no: Option<String>,
    pub department: Option<String>,
    pub owner: Option<String>,
    pub function: Option<String>,
    pub purchase_date: Option<String>,
    pub warranty_expire: Option<String>,
    pub status: Option<String>,
    pub power_watt: Option<i32>,
    /// 固有高度（U）；不传时由服务端推导：已给 U 位区间 → 区间高度；否则 → 型号高度；否则 1。
    pub height_u: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceUpdate {
    #[serde(default)] pub name: Patch<String>,
    #[serde(default)] pub device_model_id: Patch<i32>,
    #[serde(default)] pub rack_id: Patch<i32>,
    #[serde(default)] pub start_u: Patch<i32>,
    #[serde(default)] pub end_u: Patch<i32>,
    #[serde(default)] pub ip_addresses: Patch<String>,
    #[serde(default)] pub serial_no: Patch<String>,
    #[serde(default)] pub asset_no: Patch<String>,
    #[serde(default)] pub department: Patch<String>,
    #[serde(default)] pub owner: Patch<String>,
    #[serde(default)] pub function: Patch<String>,
    #[serde(default)] pub purchase_date: Patch<String>,
    #[serde(default)] pub warranty_expire: Patch<String>,
    #[serde(default)] pub status: Patch<String>,
    #[serde(default)] pub power_watt: Patch<i32>,
    /// 固有高度（U）；下架时前端不传该字段即可保留，避免重上架回退 1U。
    #[serde(default)] pub height_u: Patch<i32>,
}

/// Excel 导入结果（N-04/N-21）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`**。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportResult {
    /// 新增设备数
    pub imported: u32,
    /// 覆盖更新数（`update_mode = overwrite` 命中已存在记录）
    pub updated: u32,
    /// 跳过数（`update_mode = skip` 命中已存在记录）
    pub skipped: u32,
    /// 解析到的总数据行数
    pub total: u32,
    /// 本次新建型号数（N-21）
    pub models_created: u32,
    /// 告警（未知设备类型兜底 other、同名型号类型保持等，不阻断导入，N-21）
    pub warnings: Vec<String>,
    /// 错误明细
    pub errors: Vec<String>,
}

/// Excel 导入进度事件负载（N-06）。
///
/// 由 `import_devices_excel` 在 blocking 任务中经 `AppHandle::emit("import://progress", ..)`
/// 推送；前端 `useImportProgress` 消费。字段一律 snake_case（§8-15 N-A 裁决）。
/// `phase` 取值：`"parsing"`（解析文件） | `"importing"`（写入数据） | `"done"`（完成）。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportProgress {
    /// 已处理的数据行数
    pub processed: u32,
    /// 总数据行数（解析阶段为 0，进入导入阶段后为行数估计）
    pub total: u32,
    /// 当前阶段（parsing | importing | done）
    pub phase: String,
}

/// 设备分页查询参数（N-01/N-02）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`**（字段名本身即为 snake_case）。
/// 容器级 `#[serde(default)]`：任一字段缺失即取默认（`None`），便于前端按需只传部分条件。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct DeviceQuery {
    /// 机柜过滤
    pub rack_id: Option<i32>,
    /// 机房过滤（经 `rack_id IN (SELECT id FROM racks WHERE room_id = ?)` 生效）
    pub room_id: Option<i32>,
    /// 多字段搜索词（name / ip_addresses / serial_no / asset_no，LIKE 子串，大小写不敏感）
    pub search: Option<String>,
    /// 是否包含已软删除记录（默认 false）
    pub include_deleted: Option<bool>,
    /// 排序字段（白名单：name|ip_addresses|serial_no|asset_no|status|power_watt|created_at|updated_at|rack_id）
    pub sort_field: Option<String>,
    /// 排序方向（仅 asc|desc，默认 asc）
    pub sort_order: Option<String>,
    /// 偏移量（默认 0）
    pub offset: Option<i64>,
    /// 每页数量（默认 100，上限 1000）
    pub limit: Option<i64>,
}

/// 设备分页结果（N-01）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`**。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevicePage {
    /// 当前页设备
    pub items: Vec<Device>,
    /// 满足过滤条件的总记录数（用于前端分页器）
    pub total: i64,
}

/// Excel 导入选项（N-04/N-05）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`**。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ImportOptions {
    /// 已存在记录的更新模式：`"skip"`（跳过） | `"overwrite"`（覆盖）
    pub update_mode: String,
    /// 是否按「机房」列自动关联/创建机房（`find_or_create_room`，幂等）
    pub link_room: bool,
}

/// 批量删除结果（N-20）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`** →
/// JSON `{ "deleted": n, "not_found": [ids] }`。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeleteBatchResult {
    /// 成功（软）删除的设备数
    pub deleted: u32,
    /// 不存在 / 已被软删除而跳过的 id 列表（前端据此 `message.warning`）
    pub not_found: Vec<i32>,
}

/// 备份/恢复结果（N-18）。
///
/// 字段一律 snake_case（§8-15 N-A 裁决），**不添加 `rename_all`**。
#[allow(dead_code)] // 由 T2.6（备份恢复）消费
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RestoreResult {
    /// 是否需重启应用方可生效
    pub restart_required: bool,
    /// 面向用户的提示信息
    pub message: String,
}

/// 设备类型归一化（N-21）：接受中文名/英文枚举值，大小写不敏感、忽略空白与 `_`/`-`；
/// 未知 / 空串 → `"other"`（兜底，不阻断导入）。
///
/// 返回 8 选 1 的静态字符串，与前端 `constants/labels.ts` 的 `DEVICE_TYPES` 一一对应
/// （§8-14 双源同步）。映射表见架构设计 §3.3。
pub fn normalize_device_type(raw: &str) -> &'static str {
    // 归一化：trim → 小写 → 去除所有空白 / '_' / '-'
    let key: String = raw
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect();
    match key.as_str() {
        "server" | "服务器" => "server",
        "switch" | "交换机" => "switch",
        "router" | "路由器" => "router",
        "storage" | "存储阵列" | "存储" => "storage",
        "nas" | "nas存储" => "nas",
        "security" | "网安设备" | "安全设备" => "security",
        "loadbalancer" | "负载均衡" | "lb" => "loadbalancer",
        "other" | "其他" => "other",
        // 未知 / 空串兜底
        _ => "other",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_device_type_chinese() {
        assert_eq!(normalize_device_type("交换机"), "switch");
        assert_eq!(normalize_device_type("服务器"), "server");
        assert_eq!(normalize_device_type("路由器"), "router");
        assert_eq!(normalize_device_type("存储阵列"), "storage");
        assert_eq!(normalize_device_type("存储"), "storage");
        assert_eq!(normalize_device_type("nas存储"), "nas");
        assert_eq!(normalize_device_type("网安设备"), "security");
        assert_eq!(normalize_device_type("安全设备"), "security");
        assert_eq!(normalize_device_type("负载均衡"), "loadbalancer");
        assert_eq!(normalize_device_type("其他"), "other");
    }

    #[test]
    fn test_normalize_device_type_english() {
        assert_eq!(normalize_device_type("router"), "router");
        assert_eq!(normalize_device_type("server"), "server");
        assert_eq!(normalize_device_type("switch"), "switch");
        assert_eq!(normalize_device_type("storage"), "storage");
        assert_eq!(normalize_device_type("nas"), "nas");
        assert_eq!(normalize_device_type("security"), "security");
        assert_eq!(normalize_device_type("other"), "other");
    }

    #[test]
    fn test_normalize_device_type_case_whitespace_separators() {
        assert_eq!(normalize_device_type(" Router "), "router");
        assert_eq!(normalize_device_type("LOADBALANCER"), "loadbalancer");
        assert_eq!(normalize_device_type("load_balancer"), "loadbalancer");
        assert_eq!(normalize_device_type("Load-Balancer"), "loadbalancer");
        assert_eq!(normalize_device_type("  SWITCH  "), "switch");
        assert_eq!(normalize_device_type("lb"), "loadbalancer");
        assert_eq!(normalize_device_type("NAS"), "nas");
    }

    #[test]
    fn test_normalize_device_type_unknown_falls_back_to_other() {
        assert_eq!(normalize_device_type("防火墙"), "other");
        assert_eq!(normalize_device_type("工业网关"), "other");
        assert_eq!(normalize_device_type(""), "other");
        assert_eq!(normalize_device_type("   "), "other");
        assert_eq!(normalize_device_type("whatever"), "other");
    }

    /// 点 6：8 个英文枚举逐一自映射（补齐现有用例未显式覆盖的 loadbalancer）。
    #[test]
    fn test_normalize_device_type_all_eight_enums_self_map() {
        for t in [
            "server", "switch", "router", "storage", "nas", "security", "loadbalancer", "other",
        ] {
            assert_eq!(normalize_device_type(t), t, "英文枚举 {} 应自映射", t);
        }
    }

    /// 点 6：制表符/换行/混合分隔符（空白 + `_` + `-`）一律忽略后再匹配。
    #[test]
    fn test_normalize_device_type_tabs_and_mixed_separators() {
        assert_eq!(normalize_device_type("\tSwitch\n"), "switch");
        assert_eq!(normalize_device_type("load - balancer"), "loadbalancer");
        assert_eq!(normalize_device_type("NAS_存储"), "nas");
        // 纯空白/换行亦落到 other（设计如此）
        assert_eq!(normalize_device_type("\t\n "), "other");
    }

    #[test]
    fn test_new_dtos_serialize_snake_case() {
        // 新 DTO 一律 snake_case（§8-15 N-A），确认无意外驼峰
        let result = DeleteBatchResult { deleted: 2, not_found: vec![7, 8] };
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(json, r#"{"deleted":2,"not_found":[7,8]}"#);

        let page = DevicePage { items: vec![], total: 5 };
        let json = serde_json::to_string(&page).unwrap();
        assert_eq!(json, r#"{"items":[],"total":5}"#);

        let opts = ImportOptions { update_mode: "skip".into(), link_room: true };
        let json = serde_json::to_string(&opts).unwrap();
        assert_eq!(json, r#"{"update_mode":"skip","link_room":true}"#);

        let restore = RestoreResult { restart_required: true, message: "ok".into() };
        let json = serde_json::to_string(&restore).unwrap();
        assert_eq!(json, r#"{"restart_required":true,"message":"ok"}"#);
    }
}
