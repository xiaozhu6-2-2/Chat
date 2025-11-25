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

// 创建群聊请求模型
#[derive(Deserialize, Serialize)]
pub struct CreateGroupRequest {
    
}


