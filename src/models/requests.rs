// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// 分离模块导入
use crate::models::others::FriendRequestStatus;

// 注册请求结构体
#[derive(Deserialize, Serialize, Clone)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub username: String,
}

// 登录请求模型
#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
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

// 好友请求信息模型
#[derive(Serialize)]
pub struct FriendRequestInfo {
    pub id: i64,
    pub sender_account: String,
    pub sender_username: String,
    pub status: FriendRequestStatus,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct StartPrivateChatRequest {
    pub friend_account: String,
}

