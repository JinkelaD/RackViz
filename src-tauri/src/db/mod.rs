pub mod rooms;
pub mod racks;
pub mod devices;
pub mod device_models;
pub mod settings;

// 从 state 模块重新导出事务包装器
pub use crate::state::with_transaction;

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
