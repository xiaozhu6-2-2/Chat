// src/handlers.rs
// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use axum::extract::State;

// 分离模块导入
use crate::AppState;

// 定义输入/输出结构体
#[derive(Deserialize)]
pub struct CreateUser {
    pub username: String,
}

#[derive(Serialize)]
pub struct User {
    pub id: u64,
    pub username: String,
}

// 根路径处理函数
pub async fn root() -> &'static str {
    "Hello, World!"
}

// 用户创建处理函数
pub async fn create_user(
    // 注入状态
    State(state): State<AppState>,
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    let user = User {
        id: 1337,
        username: payload.username,
    };
    (StatusCode::CREATED, Json(user))
}