// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};

// JWT
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // 用户账号
    pub exp: usize,    // 过期时间
    pub iat: usize,    // 签发时间
}

// WebSocket消息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WsMessage {
    pub id: u64,
    pub account: String, 
    pub username: String,
    pub content: String,
    pub send_at: chrono::DateTime<chrono::Utc>,
    pub message_type: String,
}

// 好友请求状态枚举
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum FriendRequestStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}