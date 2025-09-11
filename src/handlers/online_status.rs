// 库模块导入
use axum::Json;
use axum::{
    extract::State,
    extract::Path
};
use std::collections::HashSet;

// 分离模块导入
use crate::models::others::WsMessage;
use crate::models::entities::UserOnline;
use crate::state::AppState;
use crate::handlers::other::get_username;

// 更新在线状态
pub async fn update_online_status(
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

pub async fn broadcast_online_list(
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
) -> Json<Vec<UserOnline>> {
    // 获取在线账号列表
    let account_set = {
        let online_map = state.online_users.lock().await;
        online_map.get(&room_id)
            .cloned()
            .unwrap_or_default()
    };

    // 转换为用户名和账号的列表
    let mut user_list = Vec::new();

    for account in account_set {
        if let Some(username) = get_username(&state.db_pool, &account).await {
            user_list.push(UserOnline {
                account: account.to_string(),
                username,
            });
        }
    }

    Json(user_list)
}