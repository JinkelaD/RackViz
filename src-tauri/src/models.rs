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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct Room {
    pub id: i32,
    pub name: String,
    pub location: String,
    pub sort_order: i32,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}
