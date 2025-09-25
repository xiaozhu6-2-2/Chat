// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::{
    extract::State,
    extract::Path
};
use axum::Extension;
use tokio::sync::broadcast;
use chrono::DateTime;
use chrono::Utc;

// 分离模块导入
use crate::models::others::Claims;
use crate::models::requests::StartPrivateChatRequest;
use crate::models::responses::PrivateSessionResponse;
use crate::models::entities::PrivateMessage;
use crate::state::AppState;
use crate::handlers::other::get_username;

// // 创建私聊
// pub async fn start_private_chat(
//     Extension(claims): Extension<Claims>,
//     State(state): State<AppState>,
//     Json(payload): Json<StartPrivateChatRequest>,
// ) -> Result<Json<PrivateSessionResponse>, StatusCode> {
//     let user_account = claims.sub;
//     let friend_account = payload.friend_account;

//     // 验证是否为好友关系
//     let is_friend = sqlx::query_scalar!(
//         r#"SELECT EXISTS(
//             SELECT 1 FROM friends 
//             WHERE (user_a = ? AND user_b = ?) 
//             OR (user_a = ? AND user_b = ?)
//         )"#,
//         user_account,
//         friend_account,
//         friend_account,
//         user_account
//     )
//     .fetch_one(&state.db_pool)
//     .await
//     .map(|exists: i64| exists > 0)
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     if !is_friend {
//         return Err(StatusCode::FORBIDDEN);
//     }

//     // 获取好友用户名
//     let friend_username = get_username(&state.db_pool, &friend_account)
//         .await
//         .ok_or(StatusCode::NOT_FOUND)?;

//     let friend_account_clone = friend_account.clone();

//     // 创建或获取私聊会话
//     let (user1, user2) = if user_account < friend_account {
//         (user_account, friend_account_clone)
//     } else {
//         (friend_account_clone, user_account)
//     };

//     let session = sqlx::query!(
//         r#"INSERT INTO private_chat_sessions (user1_account, user2_account)
//            VALUES (?, ?)
//            ON DUPLICATE KEY UPDATE session_id=LAST_INSERT_ID(session_id)"#,
//         user1,
//         user2
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     let session_id = session.last_insert_id() as u64;

//     // 初始化广播通道
//     {
//         let mut sessions = state.private_sessions.lock().await;
//         sessions.entry(session_id).or_insert_with(|| broadcast::channel(100).0);
//     }

//     Ok(Json(PrivateSessionResponse {
//         session_id,
//         friend_account,
//         friend_username,
//     }))
// }

// // 获取私聊历史信息
// pub async fn get_private_chat_history(
//     Path(session_id): Path<u64>,
//     State(state): State<AppState>,
//     Extension(claims): Extension<Claims>,
// ) -> Result<Json<Vec<PrivateMessage>>, StatusCode> {
//     let user_account = claims.sub;

//     // 验证会话访问权限
//     let has_access = sqlx::query_scalar!(
//         r#"SELECT EXISTS(
//             SELECT 1 FROM private_chat_sessions 
//             WHERE session_id = ? 
//             AND (user1_account = ? OR user2_account = ?)
//         )"#,
//         session_id,
//         user_account,
//         user_account
//     )
//     .fetch_one(&state.db_pool)
//     .await
//     .map(|exists: i64| exists > 0)
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     if !has_access {
//         return Err(StatusCode::FORBIDDEN);
//     }

//     // 获取历史消息
//     type PrivateMessageRow = (i64, i64, String, Option<String>, String, DateTime<Utc>);

//     let rows = sqlx::query_as::<_, PrivateMessageRow>(
//         r#"SELECT 
//             pm.message_id,
//             pm.session_id,
//             pm.sender_account,
//             ui.username AS sender_username,
//             pm.content,
//             pm.sent_at
//         FROM private_messages pm
//         JOIN user_info ui ON pm.sender_account = ui.account
//         WHERE pm.session_id = ?
//         ORDER BY pm.sent_at ASC"#
//     )
//     .bind(session_id)
//     .fetch_all(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     let messages = rows.into_iter().map(|row| {
//         PrivateMessage {
//             message_id: row.0,
//             session_id: row.1,
//             sender_account: row.2.clone(),
//             sender_username: row.3.unwrap_or(row.2.clone()),
//             content: row.4,
//             sent_at: row.5,
//         }
//     }).collect();

//     Ok(Json(messages))
// }