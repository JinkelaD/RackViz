use serde::{Deserialize, Serialize};
use chrono::NaiveDate;

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
    pub name: Option<String>,
    pub manufacturer: Option<String>,
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    pub height_u: Option<i32>,
    pub power_watt: Option<i32>,
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
    pub name: Option<String>,
    pub location: Option<String>,
    pub sort_order: Option<i32>,
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
    pub name: Option<String>,
    pub height_u: Option<i32>,
    pub row: Option<i32>,
    pub col: Option<i32>,
    pub view: Option<String>,
    pub sort_order: Option<i32>,
    pub room_id: Option<i32>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct DeviceUpdate {
    pub name: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ImportResult {
    pub imported: u32,
    pub skipped: u32,
    pub errors: Vec<String>,
}
