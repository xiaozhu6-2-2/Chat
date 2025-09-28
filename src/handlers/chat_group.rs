// // src/handlers/chat_group.rs
// /*
//     这个模块是用于处理管理群聊的操作，例如创建、删除、修改群名称、添加群管理员等等
// */
// // 库模块导入
// use axum::{
//     http::StatusCode,
//     Json,
// };
// use axum::extract::State;
// use axum::Extension;

// // 分离模块导入
// use crate::models::others::Claims;
// use crate::models::requests::{
//     CreateChatroomRequest, 
//     JoinChatroomRequest, 
//     LeaveChatroomRequest,
// };
// use crate::models::responses::{
//     ChatroomResponse, 
//     JoinedChatroomInfo,
// };
// use crate::state::AppState;
// use crate::handlers::online_status::broadcast_online_list;

// // 创建聊天室
// #[axum::debug_handler]
// pub async fn create_chatroom(
//     Extension(claims): Extension<Claims>,
//     State(state): State<AppState>,
//     Json(payload): Json<CreateChatroomRequest>,
// ) -> Result<Json<ChatroomResponse>, StatusCode> {
//     let account = claims.sub;

//     // 插入聊天室记录
//     let result = sqlx::query!(
//         "INSERT INTO chatrooms (name, created_by) VALUES (?, ?)",
//         payload.name,
//         account
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     let chatroom_id = result.last_insert_id() as u32;

//     // 自动将创建者加入聊天室
//     sqlx::query!(
//         "INSERT INTO chatroom_members (chatroom_id, account) VALUES (?, ?)",
//         chatroom_id,
//         account
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     Ok(Json(ChatroomResponse {
//         success: true,
//         chatroom_id: Some(chatroom_id),
//         message: Some("聊天室创建成功".into()),
//     }))
// }

// // 加入聊天室处理函数
// #[axum::debug_handler]
// pub async fn join_chatroom(
//     Extension(claims): Extension<Claims>,
//     State(state): State<AppState>,
//     Json(payload): Json<JoinChatroomRequest>,
// ) -> Result<Json<ChatroomResponse>, StatusCode> {
//     let account = claims.sub;
//     let chatroom_id = payload.chatroom_id;

//     // 检查聊天室是否存在
//     let chatroom_exists: Option<i64> = sqlx::query_scalar!(
//         "SELECT 1 FROM chatrooms WHERE chatroom_id = ?",
//         chatroom_id
//     )
//     .fetch_optional(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     if chatroom_exists.is_none() {
//         return Ok(Json(ChatroomResponse {
//             success: false,
//             chatroom_id: None,
//             message: Some("聊天室不存在".into()),
//         }));
//     }

//     // 检查是否已是成员
//     let is_member: Option<i64> = sqlx::query_scalar!(
//         "SELECT 1 FROM chatroom_members WHERE chatroom_id = ? AND account = ?",
//         chatroom_id,
//         account
//     )
//     .fetch_optional(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     if is_member.is_some() {
//         return Ok(Json(ChatroomResponse {
//             success: false,
//             chatroom_id: Some(chatroom_id),
//             message: Some("您已是该聊天室成员".into()),
//         }));
//     }

//     // 加入聊天室
//     sqlx::query!(
//         "INSERT INTO chatroom_members (chatroom_id, account) VALUES (?, ?)",
//         chatroom_id,
//         account
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     // 广播在线列表更新
//     broadcast_online_list(chatroom_id, &state).await;

//     Ok(Json(ChatroomResponse {
//         success: true,
//         chatroom_id: Some(chatroom_id),
//         message: Some("成功加入聊天室".into()),
//     }))
// }

// // 退出聊天室处理函数
// #[axum::debug_handler]
// pub async fn leave_chatroom(
//     Extension(claims): Extension<Claims>,
//     State(state): State<AppState>,
//     Json(payload): Json<LeaveChatroomRequest>,
// ) -> Result<Json<ChatroomResponse>, StatusCode> {
//     let account = claims.sub;
//     let chatroom_id = payload.chatroom_id;

//     // 退出聊天室
//     let result = sqlx::query!(
//         "DELETE FROM chatroom_members WHERE chatroom_id = ? AND account = ?",
//         chatroom_id,
//         account
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     if result.rows_affected() == 0 {
//         return Ok(Json(ChatroomResponse {
//             success: false,
//             chatroom_id: Some(chatroom_id),
//             message: Some("您不在该聊天室中".into()),
//         }));
//     }

//     // 更新在线状态
//     sqlx::query!(
//         "UPDATE chatroom_members SET is_online = false 
//          WHERE chatroom_id = ? AND account = ?",
//         chatroom_id,
//         account
//     )
//     .execute(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
//     // 更新内存状态
//     let mut online_map = state.online_users.lock().await;
//     if let Some(users) = online_map.get_mut(&chatroom_id) {
//         users.remove(&account);
//     }

//     // 广播在线列表更新
//     broadcast_online_list(chatroom_id, &state).await;
    
//     Ok(Json(ChatroomResponse {
//         success: true,
//         chatroom_id: Some(chatroom_id),
//         message: Some("已退出聊天室".into()),
//     }))
// }

// // 聊天室列表处理函数
// pub async fn get_joined_chatrooms(
//     Extension(claims): Extension<Claims>,
//     State(state): State<AppState>,
// ) -> Result<Json<Vec<JoinedChatroomInfo>>, StatusCode> {
//     let account = claims.sub;
    
//     // 查询用户加入的所有聊天室
//     let records = sqlx::query!(
//         r#"
//         SELECT 
//             c.chatroom_id,
//             c.name,
//             c.created_by,
//             u.username AS creator_username,
//             c.created_at
//         FROM chatroom_members cm
//         INNER JOIN chatrooms c ON cm.chatroom_id = c.chatroom_id
//         LEFT JOIN user_info u ON c.created_by = u.account
//         WHERE cm.account = ?
//         ORDER BY cm.joined_at DESC
//         "#,
//         account
//     )
//     .fetch_all(&state.db_pool)
//     .await
//     .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

//     let chatrooms = records.into_iter().map(|r| {
//         JoinedChatroomInfo {
//             chatroom_id: r.chatroom_id,
//             name: r.name,
//             created_by: r.created_by,
//             creator_username: r.creator_username,
//             created_at: r.created_at,
//         }
//     }).collect();

//     Ok(Json(chatrooms))
// }