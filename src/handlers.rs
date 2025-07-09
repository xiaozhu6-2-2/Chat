// src/handlers.rs
// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use axum::extract::State;
use sqlx::MySqlPool;
use std::error::Error;
// 分离模块导入
use crate::AppState;


// 定义登录/响应请求结构体
#[derive(Deserialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub username: String,
}

// 根路径处理函数
pub async fn root() -> &'static str {
    "Hello, World!"
}

// 登录处理函数
pub async fn login(
    State(state): State<AppState>, // 注入状态
    Json(payload): Json<LoginRequest>, // 解析为请求结构体
) -> Result<Json<LoginResponse>, StatusCode> {
    // 1. 验证账号密码是否正确
    match validate_credentials(&state.db_pool, &payload.account, &payload.password).await {
        Ok(Some(username)) => {
            // 2. 验证成功，生成响应结构体
            Ok(Json(LoginResponse {
                username,
            }))
        }
        Ok(None) => {
            // 3. 验证失败，返回 401
            Err(StatusCode::UNAUTHORIZED)
        }
        Err(_) => {
            // 4. 数据库错误，返回 500
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn validate_credentials(
    db_pool: &MySqlPool,
    account: &str,
    password: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    // 从数据库中查询对应用户账号的用户信息
    let user = sqlx::query!(
        "SELECT * FROM user_info WHERE account = ?",
        account
    )
    .fetch_one(db_pool)
    .await;

    match user {
        Ok(user) => {
            // 验证密码
            if let Some(user_password) = &user.password {
                if password == user_password {
                    Ok(Some(user.username.expect("用户ID未找到")))
                } else {
                    // 密码不匹配时的处理逻辑
                    Err("密码不匹配".to_string().into())
                }
            } else {
                // user.password 是 None 时的处理逻辑
                Err("用户密码未找到".to_string().into())
            }
        }
        Err(sqlx::Error::RowNotFound) => {
            // 用户不存在
            Ok(None)
        }
        Err(e) => {
            // 其他数据库错误
            Err(e.into())
        }
    }
}