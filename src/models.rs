// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::sync::broadcast;
use std::sync::Arc;

// 用户表模型
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct User {
    pub account: String,          // 主键 + 非空
    pub password: String,          // 非空
    pub username: Option<String>,  // 允许为空，保留Option
}

// 注册请求结构体
#[derive(Deserialize)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub username: String,
}

// 注册响应结构体
#[derive(Serialize)]
pub struct RegisterResponse {
    pub success: bool,
}

// 登录请求模型
#[derive(Deserialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

// 登录响应模型
#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String, 
    pub token: String,     // JWT令牌
}

// JWT
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // 用户账号
    pub exp: usize,    // 过期时间
    pub iat: usize,    // 签发时间
}

// 聊天室模型
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct Chatroom {
    pub chatroom_id: u32,
    pub name: String,
    pub created_by: String, // 创建者账号
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// 聊天室成员模型
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct ChatroomMember {
    pub chatroom_id: u32,
    pub account: String,
    pub joined_at: chrono::DateTime<chrono::Utc>,
}

// 创建聊天室请求
#[derive(Deserialize)]
pub struct CreateChatroomRequest {
    pub name: String,
}

// 加入聊天室请求
#[derive(Deserialize)]
pub struct JoinChatroomRequest {
    pub chatroom_id: u32,
}

// 退出聊天室请求
#[derive(Deserialize)]
pub struct LeaveChatroomRequest {
    pub chatroom_id: u32,
}

// 聊天室响应结构
#[derive(Serialize)]
pub struct ChatroomResponse {
    pub success: bool,
    pub chatroom_id: Option<u32>,
    pub message: Option<String>,
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

// WebSocket连接状态
pub struct WsConnection {
    pub sender: broadcast::Sender<WsMessage>,
    pub connections: Arc<tokio::sync::RwLock<Vec<broadcast::Sender<WsMessage>>>>,
}
