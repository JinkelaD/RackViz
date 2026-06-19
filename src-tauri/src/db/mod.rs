pub mod rooms;
pub mod racks;
pub mod devices;
pub mod device_models;
pub mod settings;

// 从 state 模块重新导出事务包装器
pub use crate::state::with_transaction;
