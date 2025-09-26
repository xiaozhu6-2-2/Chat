// // src/handlers/trans_logic.rs
// /* 
//     这个模块是用来处理前端通过websocket发来的不同类型的消息，例如私聊消息，群聊消息
//     以及后端要发送给前端的消息，例如广播群聊消息，好友上线消息
// */

// // 库模块导入
// use axum::{
//     extract::ws::{WebSocket, Message, WebSocketUpgrade},
//     extract::State,
//     extract::Path
// };
// use axum::Extension;
// use axum::response::IntoResponse;
// use tokio::sync::broadcast;
// use tracing::info;
// use futures::stream::StreamExt;
// use futures::SinkExt;
// use chrono::Utc;

// // 分离模块导入
// use crate::models::others::{Claims, WsMessage};
// use crate::models::entities::PrivateMessage;
// use crate::state::AppState;
// use crate::handlers::online_status::update_online_status;
// use crate::handlers::other::get_username;


// // WebSocket消息处理
// pub async fn handle_websocket(
//     Path(room_id): Path<u32>,
//     socket: WebSocket,
//     State(state): State<AppState>,
//     Extension(claims): Extension<Claims>, 
// ) {
//     let account = claims.sub;
    
//     // 用户上线
//     update_online_status(&state, account.clone(), room_id, true).await;

//     info!("WebSocket connected: {}", account);
    
//     // 获取或创建聊天室频道
//     let tx = {
//         let mut rooms = state.chat_rooms.lock().await;
//         rooms.entry(room_id)
//             .or_insert_with(|| broadcast::channel(100).0)
//             .clone()
//     };
    
//     // 创建接收器
//     let mut rx = tx.subscribe();
    
//     // 分离读写端
//     let (mut sender_ws, mut receiver_ws) = socket.split();

//     // 提前获取用户名
//     let username = match get_username(&state.db_pool, &account).await {
//         Some(name) => name,
//         None => account.clone(), // 如果查询失败，使用account作为回退
//     };

//     // 消息发送任务
//     let send_task = tokio::spawn({
//         async move {
//             while let Ok(msg) = rx.recv().await {
//                 let json = serde_json::to_string(&msg).unwrap();
//                 if let Err(e) = sender_ws.send(Message::Text(json.into())).await {
//                     eprintln!("WebSocket send error: {}", e);
//                     break;
//                 }
//             }
//         }
//     });
    
//     // 消息接收任务
//     let recv_task = tokio::spawn({
//         let account = account.clone();
//         let username = username.clone();
//         let db_pool = state.db_pool.clone();
//         let tx = tx.clone();
        
//         async move {
//             while let Some(Ok(Message::Text(text))) = receiver_ws.next().await {
//                 // 解析消息
//                 let now = chrono::Utc::now();
                
//                 // 存储到数据库
//                 if let Ok(result) = sqlx::query!(
//                     "INSERT INTO chat_messages (chatroom_id, sender_account, content, send_at) VALUES (?, ?, ?, ?)",
//                     room_id,
//                     account,
//                     text.to_string(),
//                     now.naive_utc()
//                 )
//                 .execute(&db_pool)
//                 .await {
//                     let message_id = result.last_insert_id() as u64;

//                     // 广播消息
//                     let ws_msg = WsMessage {
//                         id: message_id,
//                         account: account.clone(),
//                         username: username.clone(),
//                         content: text.to_string(),
//                         send_at: now,
//                         message_type: "text".to_string(),
//                     };
                    
//                     if let Err(e) = tx.send(ws_msg) {
//                         eprintln!("Broadcast error: {}", e);
//                     }
//                 } else {
//                     eprintln!("Failed to save message to database");
//                 }
//             }
//         }
//     });
    
//     // 等待任意任务结束
//     tokio::select! {
//         _ = send_task => {}
//         _ = recv_task => {}
//     }
    
//     info!("WebSocket disconnected: {}", account);

//      // 连接结束时用户下线
//     update_online_status(&state, account.clone(), room_id, false).await;
// }

// // // 私聊会话
// // pub async fn handle_private_websocket(
// //     Path(session_id): Path<u64>,
// //     ws: WebSocketUpgrade,
// //     State(state): State<AppState>,
// //     Extension(claims): Extension<Claims>,
// // ) -> impl IntoResponse {
// //     let user_account = claims.sub.clone();
    
// //     ws.on_upgrade(move |socket| async move {
// //         // 验证用户是否有权访问此会话
// //         let is_valid = sqlx::query_scalar!(
// //             r#"SELECT EXISTS(
// //                 SELECT 1 FROM private_chat_sessions 
// //                 WHERE session_id = ? 
// //                 AND (user1_account = ? OR user2_account = ?)
// //             )"#,
// //             session_id,
// //             user_account,
// //             user_account
// //         )
// //         .fetch_one(&state.db_pool)
// //         .await
// //         .map(|exists: i64| exists > 0)
// //         .unwrap_or(false);

// //         if !is_valid {
// //             return;
// //         }

// //         // 获取或创建广播通道
// //         let tx = {
// //             let mut sessions = state.private_sessions.lock().await;
// //             sessions.entry(session_id)
// //                 .or_insert_with(|| broadcast::channel(100).0)
// //                 .clone()
// //         };

// //         let mut rx = tx.subscribe();
// //         let (mut sender, mut receiver) = socket.split();

// //         // 消息接收任务
// //         let send_task = tokio::spawn(async move {
// //             while let Ok(msg) = rx.recv().await {
// //                 let json = serde_json::to_string(&msg).unwrap();
// //                 if sender.send(Message::Text(json.into())).await.is_err() {
// //                     break;
// //                 }
// //             }
// //         });

// //         // 消息发送任务
// //         let recv_task = tokio::spawn({
// //             let state = state.clone();
// //             let user_account = claims.sub.clone();
// //             async move {
// //                 while let Some(Ok(Message::Text(text))) = receiver.next().await {
// //                     // 存储私聊消息
// //                     let now = Utc::now();
// //                     let result = sqlx::query!(
// //                         "INSERT INTO private_messages (session_id, sender_account, content)
// //                          VALUES (?, ?, ?)",
// //                         session_id,
// //                         user_account,
// //                         text.to_string()
// //                     )
// //                     .execute(&state.db_pool)
// //                     .await;

// //                     if let Ok(result) = result {
// //                         let message_id = result.last_insert_id() as u64;
                        
// //                         // 获取用户名
// //                         let username = get_username(&state.db_pool, &user_account)
// //                             .await
// //                             .unwrap_or_else(|| user_account.clone());

// //                         // 广播消息
// //                         let private_msg = PrivateMessage {
// //                             message_id: message_id as i64,
// //                             session_id: session_id as i64,
// //                             sender_account: user_account.clone(),
// //                             sender_username: username,
// //                             content: text.to_string(),
// //                             sent_at: now,
// //                         };

// //                         if let Some(tx) = state.private_sessions.lock().await.get(&session_id) {
// //                             let _ = tx.send(private_msg);
// //                         }
// //                     }
// //                 }
// //             }
// //         });

// //         tokio::select! {
// //             _ = send_task => {}
// //             _ = recv_task => {}
// //         }
// //     })
// // }