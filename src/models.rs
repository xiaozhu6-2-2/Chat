// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

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
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,   // 用户账号
    pub exp: usize,    // 过期时间
    pub iat: usize,    // 签发时间
}