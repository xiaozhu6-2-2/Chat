// 从库模块导入
use axum::{http::StatusCode, Json, response::{IntoResponse, Response}};
use serde::Serialize;
use thiserror::Error;
use sqlx;
use tokio_tungstenite::tungstenite::error;

#[derive(Serialize)]
struct ErrorResponse {
    code: u16,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    details: Option<String>
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("应用状态实例创建失败")]
    StateGenerationFailure(String),
    #[error("解密失败:{0}")]
    DecryptionFailure(String),
    #[error("密码哈希解析失败:{0}")]
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
    #[error("mpcs sender发送失败")]
    MpcsSenderFailure(String),
    #[error("消息未指定接收者")]
    RecipientNotFound(String),
    #[error("序列化错误")]
    SerializeFailure(String),
    #[error("broadcast发送失败")]
    BroadcastSenderFailure(String),
    #[error("数据库连接失败")]
    DatabaseConnectionFailure(String),
    #[error("服务器启动失败")]
    ServerStartFailure(String),
    #[error("公钥转换失败")]
    PubKeyTransitionFailure(String),
    #[error("Redis连接池获取失败")]
    RedisGetConnFailure(String),
    #[error("Redis操作失败'{0}'")]
    RedisOperationFailure(String),
    #[error("雪花算法生成失败'{0}'")]
    SnowflakeFailure(String),
    #[error("群聊任务管理发生错误'{0}'")]
    TaskManagerError(String),
}

// 实现AppError转化为HTTP响应
impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message, details) = match self {
            Self::StateGenerationFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("应用状态实例创建失败:{}", fault), None),
            Self::DecryptionFailure(fault) => (StatusCode::BAD_REQUEST, format!("解密失败:{}", fault), None),
            Self::HashFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("密码哈希失败{}", fault), None),
            Self::DatabaseFailure(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库操作失败{}", e), None),
            Self::DuplicateUser(account) => (StatusCode::CONFLICT, format!("用户'{}'已存在", account), None),
            Self::UserNotFound(account) => (StatusCode::NOT_FOUND, format!("用户'{}'不存在", account), None),
            Self::InvalidPassword => (StatusCode::UNAUTHORIZED, format!("密码错误"), None),
            Self::TokenGenerationFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("JWT令牌生成失败:{}", fault), None),
            Self::MpcsSenderFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("mpcs sender发送失败:{}", fault), None),
            Self::RecipientNotFound(fault) => (StatusCode::BAD_REQUEST, format!("消息未指定接收者:{}", fault), None),
            Self::SerializeFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("序列化失败:{}", fault), None),
            Self::BroadcastSenderFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("广播失败{}", fault), None),
            Self::DatabaseConnectionFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("数据库连接失败{}", fault), None),
            Self::ServerStartFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("服务器启动失败{}", fault), None),
            Self::PubKeyTransitionFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("公钥转换失败:{}", fault), None),
            Self::RedisGetConnFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis获取连接失败{}", fault), None),
            Self::RedisOperationFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Redis操作失败{}", fault), None),
            Self::SnowflakeFailure(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("雪花算法生成失败{}", fault), None),
            Self::TaskManagerError(fault) => (StatusCode::INTERNAL_SERVER_ERROR, format!("群聊任务管理发生错误{}", fault), None),

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