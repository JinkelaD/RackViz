pub mod rooms;
pub mod racks;
pub mod devices;
pub mod device_models;
pub mod settings;

use chrono::Utc;

// 从 state 模块重新导出事务包装器
pub use crate::state::with_transaction;

/// 软删除过滤铁律（§8-2）：除回收站 / `include_deleted=true` 外，
/// 所有 list/query/count/查重 SQL 必须含 `deleted_at IS NULL`。
/// 以本常量为准，禁止手写字面量漂移。
pub const NOT_DELETED: &str = "deleted_at IS NULL";

/// 生成当前 UTC 时间戳（ISO8601，`%Y-%m-%dT%H:%M:%SZ`）。
/// 时间戳单一维护方为 Rust 层（§8-1），前端不可传入。
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// 三态字段赋值（替代无法置空的 COALESCE 部分更新）。
/// 用法：`patch_assign!(assignments, params, "col", &patch_field)`
/// - `Set(v)`  → `col = ?N`（追加参数）
/// - `Clear`   → `col = NULL`
/// - `Unset`   → 忽略该列
macro_rules! patch_assign {
    ($assignments:expr, $params:expr, $col:expr, $field:expr) => {
        if let crate::models::Patch::Set(v) = $field {
            $assignments.push(format!("{} = ?{}", $col, $params.len() + 1));
            $params.push(Box::new((*v).clone()));
        } else if matches!($field, crate::models::Patch::Clear) {
            $assignments.push(format!("{} = NULL", $col));
        }
    };
}
pub(crate) use patch_assign;
