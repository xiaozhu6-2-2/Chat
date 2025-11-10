// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// 分离模块导入
use crate::models::others::FriendRequestStatus;

// 用户表模型
#[derive(Debug, Deserialize, Serialize, FromRow, PartialEq)]
pub struct User {
    pub account: String,          // 主键 + 非空
    pub password: String,          // 非空
    pub username: Option<String>,  // 允许为空，保留Option
}

// 用户全局在线状态模型(redis)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserOnline {
    pub account: String,
    pub username: String,
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

impl std::fmt::Display for FriendRequestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            FriendRequestStatus::PENDING => write!(f, "PENDING"),
            FriendRequestStatus::ACCEPTED => write!(f, "ACCEPTED"),
            FriendRequestStatus::REJECTED => write!(f, "REJECTED"),
        }
    }
}

// 好友信息模型
#[derive(Serialize, FromRow)]
pub struct FriendInfo {
    pub account: Option<String>,
    pub username: Option<String>,
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
