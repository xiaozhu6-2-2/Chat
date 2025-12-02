// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};

// 分离模块导入

// 注册请求结构体
#[derive(Deserialize, Serialize, Clone)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub username: String,
    pub gender: String,
    pub region: String,
    pub bio: String,
    pub avator: String,
}

// 登录请求模型
#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

// 获取用户信息请求模型
#[derive(Deserialize, Serialize)]
pub struct UserInfoRequest {
    
}


