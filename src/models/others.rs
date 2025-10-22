// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use std::sync::atomic::{AtomicUsize};
use std::sync::Arc;
// 分离模块导入
use crate::models::msg_websocket::ClientMessage;
// JWT
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // 用户账号
    pub exp: usize,    // 过期时间
    pub iat: usize,    // 签发时间
}

// 广播通道结构
pub struct GroupBroadcastChannel {
    pub tx: broadcast::Sender<ClientMessage>,
    pub created_at: tokio::time::Instant,
    pub subscriber_count: Arc<AtomicUsize>
}

// 好友请求状态枚举
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum FriendRequestStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}