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

// 获取公钥响应模型
#[derive(Serialize, Deserialize)]
pub struct SessionKeyResponse {
    pub public_key: String
}

// 创建群聊响应模型
#[derive(Serialize, Deserialize)]
pub struct CreateGroupResponse {
    pub group_id: String
}