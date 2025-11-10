// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

// 注册响应结构体
#[derive(Serialize, Deserialize)]
pub struct RegisterResponse {
    pub success: bool,
}

// 登录响应模型
#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub username: String, 
    pub account: String,
    pub token: String,     // JWT令牌
}

// 会话响应模型
#[derive(Serialize, Deserialize)]
pub struct SessionKeyRespone {
    pub public_key : String
}

// 聊天室响应结构
#[derive(Serialize)]
pub struct ChatroomResponse {
    pub success: bool,
    pub chatroom_id: Option<u32>,
    pub message: Option<String>,
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