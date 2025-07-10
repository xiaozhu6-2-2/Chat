// src/handlers.rs
// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::extract::State;
use axum::Extension;
use sqlx::MySqlPool;
use std::error::Error;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher
};
use rand_core::OsRng;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};

// 分离模块导入
use crate::{models::{
        RegisterRequest, 
        RegisterResponse,
        LoginRequest, 
        LoginResponse, 
        User,
        Claims
    }, state::AppState};

use crate::models::{CreateChatroomRequest, JoinChatroomRequest, LeaveChatroomRequest, 
                   ChatroomResponse};


// 根路径处理函数
pub async fn root() -> &'static str {
    "Hello, World!"
}

// 注册处理函数
pub async fn register(
    State(state): State<AppState>,// 注入状态
    Json(payload): Json<RegisterRequest>,// 解析为请求结构体
) -> Result<Json<RegisterResponse>, StatusCode> {
    // 生成随机盐值
    let salt = SaltString::generate(&mut OsRng);
    
    // 配置Argon2参数
    let argon2 = Argon2::default();
    
    // 生成密码哈希
    let password_hash = argon2
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // 存储到数据库 (替换原有的明文存储)
    sqlx::query!(
        "INSERT INTO user_info (account, password, username) VALUES (?, ?, ?)",
        payload.account,
        password_hash,
        payload.username,
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterResponse { success: true }))
}

// 登录处理函数
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    match validate_credentials(&state.db_pool, &payload.account, &payload.password).await {
        Ok(Some(username)) => {
            // 生成JWT令牌
            let token = generate_jwt(&payload.account)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                
            Ok(Json(LoginResponse {
                username,
                token,
            }))
        }
        Ok(None) => Err(StatusCode::UNAUTHORIZED), // 认证失败， 返回401
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),// 服务器内部错误， 返回500
    }
}

// 登录验证逻辑函数
async fn validate_credentials(
    db_pool: &MySqlPool,
    account: &str,
    password: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    // 从数据库中查询用户信息
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM user_info WHERE account = ?"
    )
    .bind(account)
    .fetch_optional(db_pool)
    .await?;

    match user {
        Some(user) => {
            // 验证密码哈希
            let parsed_hash = PasswordHash::new(&user.password)
                .map_err(|_| "密码哈希解析失败")?;
                
            let argon2 = Argon2::default();
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(user.username), // 验证成功
                Err(_) => Ok(None),         // 密码不匹配
            }
        }
        None => Ok(None), // 用户不存在
    }
}

// JWT生成函数
fn generate_jwt(account: &str) -> Result<String, Box<dyn Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as usize;
    
    let exp = now + 3600; // 1小时有效期
    
    let claims = Claims {
        sub: account.to_string(),
        exp,
        iat: now,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(std::env::var("JWT_SECRET")?.as_ref())
    )?;
    
    Ok(token)
}

// 保护处理函数
pub async fn protected() -> &'static str {
    "Protected content!"
}


// 创建聊天室
#[axum::debug_handler]
pub async fn create_chatroom(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<CreateChatroomRequest>,
) -> Result<Json<ChatroomResponse>, StatusCode> {
    let account = claims.sub;

    // 插入聊天室记录
    let result = sqlx::query!(
        "INSERT INTO chatrooms (name, created_by) VALUES (?, ?)",
        payload.name,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chatroom_id = result.last_insert_id() as u32;

    // 自动将创建者加入聊天室
    sqlx::query!(
        "INSERT INTO chatroom_members (chatroom_id, account) VALUES (?, ?)",
        chatroom_id,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChatroomResponse {
        success: true,
        chatroom_id: Some(chatroom_id),
        message: Some("聊天室创建成功".into()),
    }))
}

// 加入聊天室处理函数
#[axum::debug_handler]
pub async fn join_chatroom(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<JoinChatroomRequest>,
) -> Result<Json<ChatroomResponse>, StatusCode> {
    let account = claims.sub;
    let chatroom_id = payload.chatroom_id;

    // 检查聊天室是否存在
    let chatroom_exists: Option<i64> = sqlx::query_scalar!(
        "SELECT 1 FROM chatrooms WHERE chatroom_id = ?",
        chatroom_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if chatroom_exists.is_none() {
        return Ok(Json(ChatroomResponse {
            success: false,
            chatroom_id: None,
            message: Some("聊天室不存在".into()),
        }));
    }

    // 检查是否已是成员
    let is_member: Option<i64> = sqlx::query_scalar!(
        "SELECT 1 FROM chatroom_members WHERE chatroom_id = ? AND account = ?",
        chatroom_id,
        account
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_member.is_some() {
        return Ok(Json(ChatroomResponse {
            success: false,
            chatroom_id: Some(chatroom_id),
            message: Some("您已是该聊天室成员".into()),
        }));
    }

    // 加入聊天室
    sqlx::query!(
        "INSERT INTO chatroom_members (chatroom_id, account) VALUES (?, ?)",
        chatroom_id,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ChatroomResponse {
        success: true,
        chatroom_id: Some(chatroom_id),
        message: Some("成功加入聊天室".into()),
    }))
}

// 退出聊天室处理函数
#[axum::debug_handler]
pub async fn leave_chatroom(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<LeaveChatroomRequest>,
) -> Result<Json<ChatroomResponse>, StatusCode> {
    let account = claims.sub;
    let chatroom_id = payload.chatroom_id;

    // 退出聊天室
    let result = sqlx::query!(
        "DELETE FROM chatroom_members WHERE chatroom_id = ? AND account = ?",
        chatroom_id,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Ok(Json(ChatroomResponse {
            success: false,
            chatroom_id: Some(chatroom_id),
            message: Some("您不在该聊天室中".into()),
        }));
    }

    Ok(Json(ChatroomResponse {
        success: true,
        chatroom_id: Some(chatroom_id),
        message: Some("已退出聊天室".into()),
    }))
}