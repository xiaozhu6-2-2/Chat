// 从库模块导入
use axum::{http::StatusCode, Json, response::{IntoResponse, Response}};
use serde::Serialize;
use thiserror::Error;
use sqlx;

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("解密失败:{0}")]
    DecryptionFailure(String),
    #[error("密码哈希失败:{0}")]
    HashFailure(String),
    #[error("数据库操作失败:{0}")]
    DatabaseFailure(#[from] sqlx::Error),
    #[error("用户'{0}'已存在")]
    DuplicateUser(String),
    #[error("用户'{0}'不存在")]
    UserNotFound(String),
    #[error("密码错误")]
    InvalidPassword,
    #[error("JWT令牌生成失败")]
    TokenGenerationFailure(String),
}

// 实现AppError转化为HTTP响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, details) = match self {
            Self::DecryptionFailure(fault) => (StatusCode::BAD_REQUEST, format!("解密失败:{}", fault), None),
            Self::HashFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("密码哈希失败{}", fault), None),
            Self::DatabaseFailure(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库操作失败{}", e), None),
            Self::DuplicateUser(account) => (StatusCode::CONFLICT, format!("用户'{}'已存在", account), None),
            Self::UserNotFound(account) => (StatusCode::NOT_FOUND, format!("用户'{}'不存在", account), None),
            Self::InvalidPassword => (StatusCode::UNAUTHORIZED, format!("密码错误"), None),
            Self::TokenGenerationFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("JWT令牌生成失败:{}", fault), None),
        };

        let error_response = ErrorResponse {
            code: status.as_u16(),
            message,
            details,
        };

        (status, Json(error_response)).into_response()
    }
}

// 定义新Result简化旧Result
pub type AppResult<T> = Result<T, AppError>;