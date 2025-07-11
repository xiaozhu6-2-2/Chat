// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use tokio::sync::broadcast;
use std::sync::Arc;
use chrono::{DateTime, Utc};
use chrono::NaiveDateTime;

// 用户表模型
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct User {
    pub account: String,          // 主键 + 非空
    pub password: String,          // 非空
    pub username: Option<String>,  // 允许为空，保留Option
}
// 在线用户模型
#[derive(Serialize, Deserialize)]
pub struct UserOnline {
    pub account: String,
    pub username: String,
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

// 好友请求状态枚举
#[derive(Debug, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(rename_all = "UPPERCASE")]
pub enum FriendRequestStatus {
    PENDING,
    ACCEPTED,
    REJECTED,
}

impl std::fmt::Display for FriendRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FriendRequestStatus::PENDING => write!(f, "PENDING"),
            FriendRequestStatus::ACCEPTED => write!(f, "ACCEPTED"),
            FriendRequestStatus::REJECTED => write!(f, "REJECTED"),
        }
    }
}

// 好友请求模型
#[derive(Debug, Deserialize, Serialize, FromRow)]
pub struct FriendRequest {
    pub id: i64,
    pub sender_account: String,
    pub receiver_account: String,
    pub status: FriendRequestStatus,
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    pub updated_at: Option<chrono::DateTime<chrono::Utc>>,
}

// 好友请求发送模型
#[derive(Deserialize)]
pub struct SendFriendRequest {
    pub receiver_account: String,
}

// 好友请求响应模型
#[derive(Deserialize)]
pub struct RespondToFriendRequest {
    pub request_id: i64,
    pub status: FriendRequestStatus, // 接受或拒绝
}

// 好友信息模型
#[derive(Serialize, FromRow)]
pub struct FriendInfo {
    pub account: Option<String>,
    pub username: Option<String>,
}

// 好友请求信息模型
#[derive(Serialize)]
pub struct FriendRequestInfo {
    pub id: i64,
    pub sender_account: String,
    pub sender_username: String,
    pub status: FriendRequestStatus,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrivateMessage {
    pub message_id: i64,
    pub session_id: i64, 
    pub sender_account: String,
    pub sender_username: String,
    pub content: String,
    pub sent_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct StartPrivateChatRequest {
    pub friend_account: String,
}

#[derive(Serialize)]
pub struct PrivateSessionResponse {
    pub session_id: u64,
    pub friend_account: String,
    pub friend_username: String,
}

// 聊天室列表响应模型
#[derive(Serialize)]
pub struct JoinedChatroomInfo {
    pub chatroom_id: i64,
    pub name: String,
    pub created_by: Option<String>,
    pub creator_username: Option<String>,
    pub created_at: Option<DateTime<Utc>>
}