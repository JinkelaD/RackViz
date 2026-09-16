use std::fmt;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorCode {
    NotFound,
    ValidationError,
    DatabaseError,
    IoError,
    Cancelled,
    Conflict,
    #[allow(dead_code)]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn not_found(entity: &str) -> Self {
        Self { code: ErrorCode::NotFound, message: format!("{} 不存在", entity), detail: None }
    }
    pub fn validation(msg: &str) -> Self {
        Self { code: ErrorCode::ValidationError, message: msg.to_string(), detail: None }
    }
    pub fn io(msg: &str) -> Self {
        Self { code: ErrorCode::IoError, message: msg.to_string(), detail: None }
    }
    pub fn cancelled(msg: &str) -> Self {
        Self { code: ErrorCode::Cancelled, message: msg.to_string(), detail: None }
    }
    pub fn conflict(msg: &str) -> Self {
        Self { code: ErrorCode::Conflict, message: msg.to_string(), detail: None }
    }
    /// 该错误是否属于"单条记录被规则拒绝"（校验/冲突）。
    /// 批量场景（如 Excel 导入）可据此**跳过该条并记警告**，而非让整批失败。
    #[allow(dead_code)]
    pub fn is_rejectable(&self) -> bool {
        matches!(self.code, ErrorCode::ValidationError | ErrorCode::Conflict)
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        Self {
            code: ErrorCode::DatabaseError,
            message: "数据库操作失败".into(),
            detail: Some(e.to_string()),
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self {
            code: ErrorCode::IoError,
            message: format!("文件操作失败: {}", e),
            detail: None,
        }
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self {
            code: ErrorCode::ValidationError,
            message: format!("数据格式错误: {}", e),
            detail: None,
        }
    }
}

impl From<rust_xlsxwriter::XlsxError> for AppError {
    fn from(e: rust_xlsxwriter::XlsxError) -> Self {
        Self {
            code: ErrorCode::IoError,
            message: format!("Excel操作失败: {}", e),
            detail: None,
        }
    }
}

impl From<r2d2::Error> for AppError {
    fn from(e: r2d2::Error) -> Self {
        Self {
            code: ErrorCode::DatabaseError,
            message: "连接池获取失败".into(),
            detail: Some(e.to_string()),
        }
    }
}

impl From<calamine::Error> for AppError {
    fn from(e: calamine::Error) -> Self {
        Self {
            code: ErrorCode::IoError,
            message: format!("Excel读取失败: {}", e),
            detail: None,
        }
    }
}
