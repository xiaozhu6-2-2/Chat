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
use std::collections::HashSet;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use chrono::DateTime;
use chrono::{TimeZone, Utc};

// 分离模块导入
use crate::{models::{
        RegisterRequest, 
        RegisterResponse,
        LoginRequest, 
        LoginResponse, 
        User,
        Claims,
        JoinedChatroomInfo
    }};

use crate::models::{CreateChatroomRequest, JoinChatroomRequest, LeaveChatroomRequest, 
                   ChatroomResponse};
use crate::models::{
    SendFriendRequest, FriendRequestInfo, RespondToFriendRequest, FriendRequestStatus,
    FriendInfo, 
};
use crate::{state::AppState, models::WsMessage};
use crate::models::{StartPrivateChatRequest, PrivateSessionResponse, PrivateMessage};

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

    // 广播在线列表更新
    broadcast_online_list(chatroom_id, &state).await;

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

    // 更新在线状态
    sqlx::query!(
        "UPDATE chatroom_members SET is_online = false 
         WHERE chatroom_id = ? AND account = ?",
        chatroom_id,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    // 更新内存状态
    let mut online_map = state.online_users.lock().await;
    if let Some(users) = online_map.get_mut(&chatroom_id) {
        users.remove(&account);
    }

    // 广播在线列表更新
    broadcast_online_list(chatroom_id, &state).await;
    
    Ok(Json(ChatroomResponse {
        success: true,
        chatroom_id: Some(chatroom_id),
        message: Some("已退出聊天室".into()),
    }))
}

// WebSocket消息处理
pub async fn handle_websocket(
    Path(room_id): Path<u32>,
    socket: WebSocket,
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>, 
) {
    let account = claims.sub;
    
    // 用户上线
    update_online_status(&state, account.clone(), room_id, true).await;

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

    // 提前获取用户名
    let username = match get_username(&state.db_pool, &account).await {
        Some(name) => name,
        None => account.clone(), // 如果查询失败，使用account作为回退
    };

    // 消息发送任务
    let send_task = tokio::spawn({
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
        let username = username.clone();
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

                    // 广播消息
                    let ws_msg = WsMessage {
                        id: message_id,
                        account: account.clone(),
                        username: username.clone(),
                        content: text.to_string(),
                        send_at: now,
                        message_type: "text".to_string(),
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

     // 连接结束时用户下线
    update_online_status(&state, account.clone(), room_id, false).await;
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

// 更新在线状态
async fn update_online_status(
    state: &AppState,
    account: String,
    room_id: u32,
    is_online: bool,
) {
    // 更新数据库
    let _ = sqlx::query!(
        "UPDATE chatroom_members SET is_online = ? 
         WHERE chatroom_id = ? AND account = ?",
        is_online,
        room_id,
        account
    )
    .execute(&state.db_pool)
    .await;

    // 更新内存状态
    let mut online_map = state.online_users.lock().await;
    let users = online_map.entry(room_id).or_insert_with(HashSet::new);
    
    if is_online {
        users.insert(account);
    } else {
        users.remove(&account);
    }
}

async fn broadcast_online_list(
    room_id: u32,
    state: &AppState,
) {
    let account_set = {
        let online_map = state.online_users.lock().await;
        online_map.get(&room_id)
            .cloned()
            .unwrap_or_default()
    };

    // 获取用户名列表
    let mut username_list = Vec::new();
    for account in &account_set {
        if let Some(username) = get_username(&state.db_pool, account).await {
            username_list.push(username);
        }
    }

    // 广播用户名列表
    let msg = WsMessage {
        id: 0,
        account: "system".to_string(),
        username: "System".to_string(),
        content: serde_json::to_string(&username_list).unwrap(),
        send_at: chrono::Utc::now(),
        message_type: "online_list".to_string(),
    };

    let chat_rooms = state.chat_rooms.lock().await;
    if let Some(tx) = chat_rooms.get(&room_id) {
        let _ = tx.send(msg);
    }
}

// 更新在线用户列表
pub async fn get_online_users(
    Path(room_id): Path<u32>,
    State(state): State<AppState>,
) -> Json<Vec<String>> {
    // 获取在线账号列表
    let account_set = {
        let online_map = state.online_users.lock().await;
        online_map.get(&room_id)
            .cloned()
            .unwrap_or_default()
    };

    // 转换为用户名列表
    let mut username_list = Vec::new();
    
    for account in account_set {
        if let Some(username) = get_username(&state.db_pool, &account).await {
            username_list.push(username);
        }
    }
    
    Json(username_list)
}

// 发送好友请求
pub async fn send_friend_request(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<SendFriendRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let sender_account = claims.sub;
    let receiver_account = payload.receiver_account;

    // 不能添加自己为好友
    if sender_account == receiver_account {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 检查是否已是好友
    let is_friend: Option<i64> = sqlx::query_scalar!(
        r#"SELECT 1 FROM friends 
           WHERE (user_a = ? AND user_b = ?) 
           OR (user_a = ? AND user_b = ?)"#,
        sender_account,
        receiver_account,
        receiver_account,
        sender_account
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_friend.is_some() {
        return Ok(Json(RegisterResponse { success: false }));
    }

    // 检查是否已有待处理请求
    let existing_request: Option<i64> = sqlx::query_scalar!(
        r#"SELECT 1 FROM friend_requests 
           WHERE sender_account = ? AND receiver_account = ? AND status = 'PENDING'"#,
        sender_account,
        receiver_account
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing_request.is_some() {
        return Ok(Json(RegisterResponse { success: false }));
    }

    // 插入新请求
    sqlx::query!(
        r#"INSERT INTO friend_requests 
           (sender_account, receiver_account, status) 
           VALUES (?, ?, 'PENDING')"#,
        sender_account,
        receiver_account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterResponse { success: true }))
}

// 列出好友请求（接收和发送的）
pub async fn list_friend_requests(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FriendRequestInfo>>, StatusCode> {
    let account = claims.sub;

    // 查询所有相关请求
    let requests = sqlx::query!(
        r#"SELECT 
            id, 
            sender_account, 
            receiver_account, 
            status as "status: FriendRequestStatus",
            created_at 
        FROM friend_requests 
        WHERE sender_account = ? OR receiver_account = ?"#,
        account,
        account
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取用户名信息
    let mut result = Vec::new();
    for req in requests {
        let sender_username = get_username(&state.db_pool, &req.sender_account)
            .await
            .unwrap_or_default();

        result.push(FriendRequestInfo {
            id: req.id,
            sender_account: req.sender_account.clone(),
            sender_username,
            status: req.status.expect("REASON"),
            created_at: req.created_at,
        });
    }

    Ok(Json(result))
}

// 响应好友请求
pub async fn respond_friend_request(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<RespondToFriendRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let account = claims.sub;
    
    // 获取请求
     let request = sqlx::query!(
        r#"SELECT 
            id, 
            sender_account, 
            receiver_account, 
            status as "status: FriendRequestStatus",
            created_at 
        FROM friend_requests WHERE id = ?"#,
        payload.request_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // 验证请求接收者
    if request.receiver_account != account {
        return Err(StatusCode::FORBIDDEN);
    }

    // 更新请求状态
    sqlx::query!(
        "UPDATE friend_requests SET status = ? WHERE id = ?",
        payload.status.to_string(),
        payload.request_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 如果接受请求，添加好友关系
    if payload.status == FriendRequestStatus::ACCEPTED {
        // 确保顺序：user_a < user_b
        let (user_a, user_b) = if request.sender_account < request.receiver_account {
            (request.sender_account.clone(), request.receiver_account.clone())
        } else {
            (request.receiver_account.clone(), request.sender_account.clone())
        };

        // 添加好友关系
        sqlx::query!(
            "INSERT INTO friends (user_a, user_b) VALUES (?, ?)",
            user_a,
            user_b
        )
        .execute(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(RegisterResponse { success: true }))
}

// 列出所有好友
pub async fn list_friends(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FriendInfo>>, StatusCode> {
    let account = claims.sub;

    // 查询好友
    let friends = sqlx::query_as!(
        FriendInfo,
        r#"SELECT 
            CASE 
                WHEN user_a = ? THEN user_b 
                ELSE user_a 
            END AS account,
            u.username
           FROM friends f
           JOIN user_info u ON 
                (f.user_a = u.account OR f.user_b = u.account) AND u.account != ?
           WHERE user_a = ? OR user_b = ?"#,
        account,
        account,
        account,
        account
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(friends))
}

// 删除好友
pub async fn remove_friend(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(friend_account): Path<String>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let account = claims.sub;

    // 删除好友关系
    let result = sqlx::query!(
        r#"DELETE FROM friends 
           WHERE (user_a = ? AND user_b = ?)
           OR (user_a = ? AND user_b = ?)"#,
        account,
        friend_account,
        friend_account,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() > 0 {
        Ok(Json(RegisterResponse { success: true }))
    } else {
        Ok(Json(RegisterResponse { success: false }))
    }
}

// 创建私聊
pub async fn start_private_chat(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<StartPrivateChatRequest>,
) -> Result<Json<PrivateSessionResponse>, StatusCode> {
    let user_account = claims.sub;
    let friend_account = payload.friend_account;

    // 验证是否为好友关系
    let is_friend = sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM friends 
            WHERE (user_a = ? AND user_b = ?) 
            OR (user_a = ? AND user_b = ?)
        )"#,
        user_account,
        friend_account,
        friend_account,
        user_account
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|exists: i64| exists > 0)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !is_friend {
        return Err(StatusCode::FORBIDDEN);
    }

    // 获取好友用户名
    let friend_username = get_username(&state.db_pool, &friend_account)
        .await
        .ok_or(StatusCode::NOT_FOUND)?;

    let friend_account_clone = friend_account.clone();

    // 创建或获取私聊会话
    let (user1, user2) = if user_account < friend_account {
        (user_account, friend_account_clone)
    } else {
        (friend_account_clone, user_account)
    };

    let session = sqlx::query!(
        r#"INSERT INTO private_chat_sessions (user1_account, user2_account)
           VALUES (?, ?)
           ON DUPLICATE KEY UPDATE session_id=LAST_INSERT_ID(session_id)"#,
        user1,
        user2
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let session_id = session.last_insert_id() as u64;

    // 初始化广播通道
    {
        let mut sessions = state.private_sessions.lock().await;
        sessions.entry(session_id).or_insert_with(|| broadcast::channel(100).0);
    }

    Ok(Json(PrivateSessionResponse {
        session_id,
        friend_account,
        friend_username,
    }))
}


// 私聊会话
pub async fn handle_private_websocket(
    Path(session_id): Path<u64>,
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_account = claims.sub.clone();
    
    ws.on_upgrade(move |socket| async move {
        // 验证用户是否有权访问此会话
        let is_valid = sqlx::query_scalar!(
            r#"SELECT EXISTS(
                SELECT 1 FROM private_chat_sessions 
                WHERE session_id = ? 
                AND (user1_account = ? OR user2_account = ?)
            )"#,
            session_id,
            user_account,
            user_account
        )
        .fetch_one(&state.db_pool)
        .await
        .map(|exists: i64| exists > 0)
        .unwrap_or(false);

        if !is_valid {
            return;
        }

        // 获取或创建广播通道
        let tx = {
            let mut sessions = state.private_sessions.lock().await;
            sessions.entry(session_id)
                .or_insert_with(|| broadcast::channel(100).0)
                .clone()
        };

        let mut rx = tx.subscribe();
        let (mut sender, mut receiver) = socket.split();

        // 消息接收任务
        let send_task = tokio::spawn(async move {
            while let Ok(msg) = rx.recv().await {
                let json = serde_json::to_string(&msg).unwrap();
                if sender.send(Message::Text(json.into())).await.is_err() {
                    break;
                }
            }
        });

        // 消息发送任务
        let recv_task = tokio::spawn({
            let state = state.clone();
            let user_account = claims.sub.clone();
            async move {
                while let Some(Ok(Message::Text(text))) = receiver.next().await {
                    // 存储私聊消息
                    let now = Utc::now();
                    let result = sqlx::query!(
                        "INSERT INTO private_messages (session_id, sender_account, content)
                         VALUES (?, ?, ?)",
                        session_id,
                        user_account,
                        text.to_string()
                    )
                    .execute(&state.db_pool)
                    .await;

                    if let Ok(result) = result {
                        let message_id = result.last_insert_id() as u64;
                        
                        // 获取用户名
                        let username = get_username(&state.db_pool, &user_account)
                            .await
                            .unwrap_or_else(|| user_account.clone());

                        // 广播消息
                        let private_msg = PrivateMessage {
                            message_id: message_id as i64,
                            session_id: session_id as i64,
                            sender_account: user_account.clone(),
                            sender_username: username,
                            content: text.to_string(),
                            sent_at: now,
                        };

                        if let Some(tx) = state.private_sessions.lock().await.get(&session_id) {
                            let _ = tx.send(private_msg);
                        }
                    }
                }
            }
        });

        tokio::select! {
            _ = send_task => {}
            _ = recv_task => {}
        }
    })
}

// 获取私聊历史信息
pub async fn get_private_chat_history(
    Path(session_id): Path<u64>,
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> Result<Json<Vec<PrivateMessage>>, StatusCode> {
    let user_account = claims.sub;

    // 验证会话访问权限
    let has_access = sqlx::query_scalar!(
        r#"SELECT EXISTS(
            SELECT 1 FROM private_chat_sessions 
            WHERE session_id = ? 
            AND (user1_account = ? OR user2_account = ?)
        )"#,
        session_id,
        user_account,
        user_account
    )
    .fetch_one(&state.db_pool)
    .await
    .map(|exists: i64| exists > 0)
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if !has_access {
        return Err(StatusCode::FORBIDDEN);
    }

    // 获取历史消息
    type PrivateMessageRow = (i64, i64, String, Option<String>, String, DateTime<Utc>);

    let rows = sqlx::query_as::<_, PrivateMessageRow>(
        r#"SELECT 
            pm.message_id,
            pm.session_id,
            pm.sender_account,
            ui.username AS sender_username,
            pm.content,
            pm.sent_at
        FROM private_messages pm
        JOIN user_info ui ON pm.sender_account = ui.account
        WHERE pm.session_id = ?
        ORDER BY pm.sent_at ASC"#
    )
    .bind(session_id)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let messages = rows.into_iter().map(|row| {
        PrivateMessage {
            message_id: row.0,
            session_id: row.1,
            sender_account: row.2.clone(),
            sender_username: row.3.unwrap_or(row.2.clone()),
            content: row.4,
            sent_at: row.5,
        }
    }).collect();

    Ok(Json(messages))
}

// 聊天室列表处理函数
pub async fn get_joined_chatrooms(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<JoinedChatroomInfo>>, StatusCode> {
    let account = claims.sub;
    
    // 查询用户加入的所有聊天室
    let records = sqlx::query!(
        r#"
        SELECT 
            c.chatroom_id,
            c.name,
            c.created_by,
            u.username AS creator_username,
            c.created_at
        FROM chatroom_members cm
        INNER JOIN chatrooms c ON cm.chatroom_id = c.chatroom_id
        LEFT JOIN user_info u ON c.created_by = u.account
        WHERE cm.account = ?
        ORDER BY cm.joined_at DESC
        "#,
        account
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let chatrooms = records.into_iter().map(|r| {
        JoinedChatroomInfo {
            chatroom_id: r.chatroom_id,
            name: r.name,
            created_by: r.created_by,
            creator_username: r.creator_username,
            created_at: r.created_at,
        }
    }).collect();

    Ok(Json(chatrooms))
}