// src/handlers.rs
// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::{
    extract::ws::{WebSocket, Message},
    extract::State,
    extract::Path
};
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
use tokio::sync::broadcast;
use tracing::info;
use futures::stream::StreamExt;
use futures::SinkExt;

// 分离模块导入
use crate::{models::{
        RegisterRequest, 
        RegisterResponse,
        LoginRequest, 
        LoginResponse, 
        User,
        Claims
    }};

use crate::models::{CreateChatroomRequest, JoinChatroomRequest, LeaveChatroomRequest, 
                   ChatroomResponse};

use crate::{state::AppState, models::WsMessage};

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

// WebSocket消息处理
pub async fn handle_websocket(
    Path(room_id): Path<u32>,
    mut socket: WebSocket,
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>, 
) {
    let account = claims.sub;
    
    info!("WebSocket connected: {}", account);
    
    // 获取或创建聊天室频道
    let tx = {
        let mut rooms = state.chat_rooms.lock().await;
        rooms.entry(room_id)
            .or_insert_with(|| broadcast::channel(100).0)
            .clone()
    };
    
    // 创建接收器
    let mut rx = tx.subscribe();
    
    // 分离读写端
    let (mut sender_ws, mut receiver_ws) = socket.split();

    // 消息发送任务
    let send_task = tokio::spawn({
        let account = account.clone();
        
        async move {
            while let Ok(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if let Err(e) = sender_ws.send(Message::Text(json.into())).await {
                    eprintln!("WebSocket send error: {}", e);
                    break;
                }
            }
        }
    });
    
    // 消息接收任务
    let recv_task = tokio::spawn({
        let account = account.clone();
        let db_pool = state.db_pool.clone();
        let tx = tx.clone();
        
        async move {
            while let Some(Ok(Message::Text(text))) = receiver_ws.next().await {
                // 解析消息
                let now = chrono::Utc::now();
                
                // 存储到数据库
                if let Ok(result) = sqlx::query!(
                    "INSERT INTO chat_messages (chatroom_id, sender_account, content, send_at) VALUES (?, ?, ?, ?)",
                    room_id,
                    account,
                    text.to_string(),
                    now.naive_utc()
                )
                .execute(&db_pool)
                .await {
                    let message_id = result.last_insert_id() as u64;
                    let username = match get_username(&state.db_pool, &account).await {
                        Some(name) => name,
                        None => account.clone(), // 如果查询失败，使用account作为回退
                    };

                    // 广播消息
                    let ws_msg = WsMessage {
                        id: message_id,
                        account: account.clone(),
                        username: username.clone(),
                        content: text.to_string(),
                        send_at: now,
                    };
                    
                    if let Err(e) = tx.send(ws_msg) {
                        eprintln!("Broadcast error: {}", e);
                    }
                } else {
                    eprintln!("Failed to save message to database");
                }
            }
        }
    });
    
    // 等待任意任务结束
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
    
    info!("WebSocket disconnected: {}", account);
}

// 查询用户名
async fn get_username(db_pool: &MySqlPool, account: &str) -> Option<String> {
    sqlx::query_scalar!(
        "SELECT username FROM user_info WHERE account = ?",
        account
    )
    .fetch_optional(db_pool)
    .await
    .unwrap_or(None)
    .flatten()
}