use std::sync::Arc;

// // src/handlers/trans_logic.rs
// /*
//     这个模块是用来处理前端通过websocket发来的不同类型的消息，例如私聊消息，群聊消息
//     以及后端要发送给前端的消息，例如广播群聊消息，好友上线消息
// */
// 库模块导入
use axum::extract::ws::{close_code, CloseFrame, Message};
use dashmap::DashMap;
use log::{error, info, warn};
use scopeguard::guard;
use serde_json::json;
use tokio::sync::broadcast;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
// 分离模块导入
use crate::models::errors::{AppError, AppResult};
use crate::models::msg_websocket::{ClientMessage, MesPayload, ServerMessage, MessageAck};
use crate::models::entities::{PrivateMessage, PrivateMsgType, GroupMsgType, EnumConvertible, AccessLevel, AccessTarget, AssociationType};
use crate::models::others::GroupBroadcastChannel;
use crate::models::repository::{UserRepository, FriendshipRepository, GroupChatRepository, PrivateChatRepository, GroupMessageRepository, FileRepository};
use crate::state::AppState;

// 回复pong
pub async fn send_pong(
    account: String,
    state: AppState
) -> AppResult<()> {
    // 构建自定义pong
    let pong = ClientMessage::Pong {
        timestamp : Some(chrono::Utc::now().timestamp()),
        data : Some(json!({"source" : "server"}))
    };

    // 构建websocket文本消息
    let ws_pong = Message::Text(
        serde_json::to_string(&pong)
        .map_err(|e| AppError::SerializeFailure(e.to_string()))?
        .into()
    );

    // 获取tx
    let tx = state.connection_pool.get(&account).map({|guard|
        guard.value().clone()// 克隆tx并释放锁
    });

    // 找到对应的连接
    if let Some(tx) = tx{
        tx.send(ws_pong).map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
        info!("成功发送Pong到{}", account);
    } else {
        warn!("{}账号没有连接", account);
    }
    
    Ok(())
}

// 发送close
pub async fn send_close(
    account: String,
    state: AppState
) -> AppResult<()> {
    // 构建WebSocket关闭帧消息
    let ws_close = Message::Close(Some(CloseFrame {
        code: close_code::NORMAL,
        reason: "Null".to_string().into()
    }));

    // 获取tx（花括号是为了释放锁）
    let tx = state.connection_pool.get(&account).map({|guard|
        guard.value().clone()// 克隆tx并释放锁
    });

    // 找到对应连接
    if let Some(tx) = tx{
        tx.send(ws_close).map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
        info!("成功发送close到{}", account);
    } else {
        warn!("{}账号没有连接", account);
    }
    
    Ok(())
}

// 处理私聊消息（增强版）
pub async fn handle_private_chat(
    payload: MesPayload,
    state: AppState,
    sender_uid: String,
) -> AppResult<()> {
    // 1. 验证payload中的sender_id是否与连接的uid一致
    if let Some(payload_sender_id) = &payload.sender_id {
        if payload_sender_id != &sender_uid {
            return Err(AppError::Forbidden("发送者ID不匹配".to_string()));
        }
    }

    // 2. 获取接收者UID
    let receiver_id = payload.get_receiver_id()
        .ok_or_else(|| AppError::RecipientNotFound("接收者UID为空".to_string()))?
        .clone();

    // 3. 获取payload中的chat_id (PID)
    let payload_chat_id = payload.chat_id.clone()
        .ok_or_else(|| AppError::NotFound("chat_id为空".to_string()))?;

    // 4. 验证权限
    state.db_pool.validate_private_message_permission(&sender_uid, &receiver_id).await?;

    // 4a. 处理媒体消息类型（file, image, voice, video, emoji）
    if let Some(content_type) = &payload.content_type {
        // 检查是否为包含file_id的媒体类型
        if matches!(content_type.as_str(), "file" | "image" | "voice" | "video" | "emoji") {
            // 从消息内容中提取file_id
            let file_id = payload.details.as_ref()
                .ok_or_else(|| AppError::BadRequest(
                    format!("{}消息缺少文件ID", content_type)
                ))?
                .clone();

            // 验证发送者是否有文件的Share权限
            let has_share_permission = state.db_pool
                .verify_file_permission(&file_id, &sender_uid, AccessLevel::Share)
                .await?;

            if !has_share_permission {
                return Err(AppError::Forbidden(
                    "您没有权限分享此文件".to_string()
                ));
            }
        }
    }

    // 5. 通过sender和receiver验证chat_id的正确性
    let private_chat = state.db_pool
        .find_chat_by_users(&sender_uid, &receiver_id)
        .await?
        .ok_or_else(|| AppError::NotFound("私聊会话不存在".to_string()))?;

    // 验证payload中的chat_id是否与数据库中的PID一致
    if private_chat.pid != payload_chat_id {
        return Err(AppError::Forbidden("chat_id验证失败，不匹配发送者和接收者的私聊".to_string()));
    }

    let chat_id = private_chat.pid;

    // 6. 生成消息ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let message_id = snowflake.next_id()?.to_string();

    // 7. 获取接收者的account（用于检查在线状态和发送消息）
    let receiver_user = state.db_pool.find_user_by_uid(&receiver_id).await?;
    let receiver_account = receiver_user.account;

    // 8. 构建消息实体（send_time设为None，让数据库使用默认值）
    // 先克隆需要使用的字段
    let content = payload.details.clone().unwrap_or_default();
    let content_type = payload.content_type.clone();
    let temp_message_id = payload.message_id.clone().unwrap_or_default();
    let message = PrivateMessage {
        msg_id: message_id.clone(),
        pid: chat_id,
        content,
        sender_uid: sender_uid.clone(),
        send_time: None, // 让数据库使用DEFAULT CURRENT_TIMESTAMP
        is_revoked: Some(0),
        is_read: Some(0),
        mes_type: parse_message_type(&content_type),
    };

    // 9. 保存消息到数据库
    PrivateChatRepository::save_message(&state.db_pool, message).await?;

    // 9a. 为媒体消息创建文件关联并授予权限
    if let Some(content_type) = &payload.content_type {
        // 检查是否为包含file_id的媒体类型
        if matches!(content_type.as_str(), "file" | "image" | "voice" | "video" | "emoji") {
            if let Some(file_id) = &payload.details {
                // 为接收者授予Share权限（包含View和Download）
                state.db_pool
                    .grant_file_permission(
                        &file_id,
                        AccessTarget::User,
                        Some(receiver_id.clone()),
                        AccessLevel::Share,
                        &sender_uid,
                        None // 无过期时间
                    )
                    .await?;

                // 创建文件与消息的关联
                state.db_pool
                    .create_file_association(
                        &file_id,
                        AssociationType::PrivateMessage,
                        &message_id,
                        &sender_uid
                    )
                    .await?;
            }
        }
    }

    // 10. 从数据库获取刚刚保存的消息（获取数据库生成的时间戳）
    let saved_message = PrivateChatRepository::find_message_by_id(&state.db_pool, &message_id).await?
        .ok_or_else(|| AppError::NotFound("消息保存失败".to_string()))?;

    // 11. 获取数据库生成的时间戳
    let timestamp = saved_message.send_time
        .ok_or_else(|| AppError::NotFound("消息时间戳缺失".to_string()))?
        .timestamp();

    // 12. 获取发送者account（用于发送ACK）
    let sender_account = state.db_pool
        .find_user_by_uid(&sender_uid).await?
        .account;

    // 13. 检查接收者是否在线（直接检查WebSocket连接池）
    let is_receiver_online = state.connection_pool.contains_key(&receiver_account);

    // 14. 发送消息（在线）或保存离线消息
    if is_receiver_online {
        // 在线 - 直接发送
        // 创建包含正式 message_id 和准确时间戳的 payload
        let mut online_payload = payload.clone();
        online_payload.message_id = Some(message_id.clone());
        online_payload.set_timestamp(Some(timestamp));
        send_private_message_online(online_payload, receiver_account, state.clone()).await?;
    }
    // 离线 - 消息已保存到数据库，无需额外操作

    // 15. 发送ACK给发送方
    send_message_ack(sender_account, MessageAck {
        temp_message_id,
        message_id,
        timestamp,
    }, state).await?;

    Ok(())
}

// 处理群聊消息（增强版）
pub async fn handle_group_chat(
    payload: MesPayload,
    state: AppState,
    sender_uid: String,
) -> AppResult<()> {
    // 1. 验证payload中的sender_id是否与连接的uid一致
    if let Some(payload_sender_id) = &payload.sender_id {
        if payload_sender_id != &sender_uid {
            return Err(AppError::Forbidden("发送者ID不匹配".to_string()));
        }
    }

    // 2. 获取群ID和chat_id
    let receiver_id = payload.get_receiver_id()
        .ok_or_else(|| AppError::RecipientNotFound("接收者ID为空".to_string()))?
        .clone();

    let chat_id = payload.chat_id.clone()
        .ok_or_else(|| AppError::NotFound("chat_id为空".to_string()))?;

    // 3. 验证receiver_id和chat_id是否相同
    if receiver_id != chat_id {
        return Err(AppError::Forbidden("群聊消息中receiver_id和chat_id必须相同".to_string()));
    }

    // 4. 验证群成员权限
    state.db_pool.validate_group_message_permission(&sender_uid, &chat_id).await?;

    // 4a. 处理群聊媒体消息类型（file, image, voice, video, emoji）
    if let Some(content_type) = &payload.content_type {
        // 检查是否为包含file_id的媒体类型
        if matches!(content_type.as_str(), "file" | "image" | "voice" | "video" | "emoji") {
            // 从消息内容中提取file_id
            let file_id = payload.details.as_ref()
                .ok_or_else(|| AppError::BadRequest(
                    format!("群聊{}消息缺少文件ID", content_type)
                ))?
                .clone();

            // 验证发送者是否有文件的Share权限
            let has_share_permission = state.db_pool
                .verify_file_permission(&file_id, &sender_uid, AccessLevel::Share)
                .await?;

            if !has_share_permission {
                return Err(AppError::Forbidden(
                    "您没有权限在群聊中分享此文件".to_string()
                ));
            }
        }
    }

    // 5. 生成消息ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let message_id = snowflake.next_id()?.to_string();

    // 6. 构建群聊消息实体（send_time设为None，让数据库使用默认值）
    // 先克隆需要使用的字段
    let content = payload.details.clone().unwrap_or_default();
    let content_type = payload.content_type.clone();
    let mentioned_uids = payload.mentioned_uids.clone();
    let quote_msg_id = payload.quote_msg_id.clone();
    let is_announcement = payload.is_announcement;
    let temp_message_id = payload.message_id.clone().unwrap_or_default();

    let message = crate::models::entities::GroupMessage {
        msg_id: message_id.clone(),
        gid: chat_id.clone(),
        content,
        sender_uid: sender_uid.clone(),
        send_time: None, // 让数据库使用DEFAULT CURRENT_TIMESTAMP
        is_revoked: Some(0),
        msg_type: parse_group_message_type(&content_type),
        mentioned_uids: if mentioned_uids.is_empty() {
            None
        } else {
            Some(serde_json::to_value(mentioned_uids).unwrap())
        },
        quote_msg_id: if quote_msg_id.is_empty() {
            None
        } else {
            Some(quote_msg_id)
        },
        is_announcement: if is_announcement { Some(1) } else { Some(0) },
    };

    // 7. 保存消息到数据库
    GroupMessageRepository::save_message(&state.db_pool, message).await?;

    // 7a. 为群聊媒体消息创建文件关联并授予群组权限
    if let Some(content_type) = &payload.content_type {
        // 检查是否为包含file_id的媒体类型
        if matches!(content_type.as_str(), "file" | "image" | "voice" | "video" | "emoji") {
            if let Some(file_id) = &payload.details {
                // 为整个群组授予Share权限（群组成员通过群组成员身份自动获得访问权限）
                state.db_pool
                    .grant_file_permission(
                        &file_id,
                        AccessTarget::Group,
                        Some(chat_id.clone()), // chat_id 就是 gid
                        AccessLevel::Share,
                        &sender_uid,
                        None // 无过期时间
                    )
                    .await?;

                // 创建文件与群聊消息的关联
                state.db_pool
                    .create_file_association(
                        &file_id,
                        AssociationType::GroupMessage,
                        &message_id,
                        &sender_uid
                    )
                    .await?;
            }
        }
    }

    // 8. 从数据库获取刚刚保存的消息（获取数据库生成的时间戳）
    let saved_message = GroupMessageRepository::find_message_by_id(&state.db_pool, &message_id).await?
        .ok_or_else(|| AppError::NotFound("消息保存失败".to_string()))?;

    // 9. 获取数据库生成的时间戳
    let timestamp = saved_message.send_time
        .ok_or_else(|| AppError::NotFound("消息时间戳缺失".to_string()))?
        .timestamp();

    // 10. 获取发送者account
    let sender_account = state.db_pool
        .find_user_by_uid(&sender_uid).await?
        .account;

    // 11. 广播消息到群聊频道
    // 创建包含正式 message_id 和准确时间戳的 payload
    let mut broadcast_payload = payload.clone();
    broadcast_payload.message_id = Some(message_id.clone());
    broadcast_payload.set_timestamp(Some(timestamp));
    send_group_message_broadcast(broadcast_payload, state.clone()).await?;

    // 12. 发送ACK给发送方
    send_message_ack(sender_account, MessageAck {
        temp_message_id,
        message_id,
        timestamp,
    }, state).await?;

    Ok(())
}

// 群聊监听任务
pub async fn group_channel_listen(
    gid : String,
    account : String,
    tx : UnboundedSender<Message>,
    broadcast_pool : Arc<DashMap<String, GroupBroadcastChannel>>,
    cancel_token: CancellationToken,
) {
    // debug
    info!("监听群聊{}", gid);
    if tx.is_closed() {
        error!("tx已关闭，监听群聊{}失败", gid);
    }

    if cancel_token.is_cancelled() {
        error!("取消令牌意外触发，监听群聊{}失败", gid);
    }

    // debug
    info!("准备创建群聊频道");

    if broadcast_pool.contains_key(&gid) {
        info!("群聊频道{}已存在，不需要创建", gid);
    }
    else {
        info!("群聊{}频道不存在于broadcast_pool中，准备创建", gid);
    }

    // 获取/创建群聊频道
    let channel = broadcast_pool.entry(gid.clone())
        .or_insert_with(|| {
            info!("创建新的群聊频道: {}", gid);
            let (broadcast_tx, _) = tokio::sync::broadcast::channel(1000);
            GroupBroadcastChannel { 
                tx: broadcast_tx, 
                created_at: tokio::time::Instant::now(), 
                subscriber_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)) 
            }    
        }).clone();

    info!("获取群聊频道成功");

    // 订阅群聊频道
    let mut rx = channel.tx.subscribe();
    
    // 增加订阅者计数
    let old_count = channel.subscriber_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    info!("用户 {} 开始监听群聊 {} 频道，当前订阅者数量: {}", account, gid, old_count + 1);
    
    // 减少计数
    let account_for_guard = account.clone();
    let gid_for_guard = gid.clone();
    let _guard = guard((), move |_| {
        // 先获取读锁，读取订阅者数量
        let should_remove = if let Some(channel) = broadcast_pool.get(&gid_for_guard) {
            let count = channel.subscriber_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            info!("用户 {} 退出群聊 {} 频道，剩余订阅者: {}", account_for_guard, gid_for_guard, count - 1);

            // 返回是否需要删除频道（当减1后为0时）
            count <= 1
        } else {
            info!("群聊频道 {} 已不存在，无需清理", gid_for_guard);
            false
        };

        // 如果需要删除频道，在锁外进行删除操作
        if should_remove {
            info!("尝试清理空闲群聊频道: {}", gid_for_guard);
            match broadcast_pool.remove(&gid_for_guard) {
                Some((_key, _channel)) => {
                    info!("成功清理空闲群聊频道: {}", gid_for_guard);
                }
                None => {
                    info!("群聊频道 {} 已被其他线程清理", gid_for_guard);
                }
            }
        }
    });

    // 监听群聊频道
    loop {
        // 检查取消令牌
        if cancel_token.is_cancelled() {
            break;
        }
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        // 检查取消令牌
                        if cancel_token.is_cancelled() {
                            break;
                        }
                        // 将收到的消息序列化
                        let msg_json = match serde_json::to_string(&msg) {
                            Ok(msg) => msg,
                            Err(e) => {
                                error!("序列化失败: {}", e);
                                break;
                            }
                        };
                        // 检查取消令牌
                        if cancel_token.is_cancelled() {
                            break;
                        }
                        // 向mpsc中发送消息
                        if tx.send(Message::Text(msg_json.into())).is_err() {
                            error!("向用户 {} 转发群聊 {} 消息失败", account, gid);
                            break;
                        }
                    },
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        warn!("用户 {} 群聊 {} 消息滞后，跳过 {} 条消息", account, gid, skipped);
                    },
                    Err(broadcast::error::RecvError::Closed) => {
                        info!("群聊 {} 频道已关闭", gid);
                        break;
                    }
                }
            },
            _ = cancel_token.cancelled() => {
                info!("群聊 {} 监听被主动取消", gid);
                break;
            }
        }
    }
    
}

// 发送上线消息
pub async fn send_online_state(
    to_uid: String,
    online_state: ServerMessage,
    state: AppState,
) -> AppResult<()> {
    // 转换为Message
    let online_state_msg = serde_json::to_string(&online_state).map_err(|e| AppError::SerializeFailure(e.to_string()))?;

    // 获取接收者的account
    let account = state.db_pool.find_user_by_uid(&to_uid).await.map(|user| user.account)?;

    // 获取tx
    let tx = state.connection_pool.get(&account).map(|guard| {
        guard.value().clone() // 获取tx，并释放锁
    });

    //发送消息
    if let Some(tx) = tx {
        tx.send(Message::Text(online_state_msg.into())).map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
        info!("成功发送在线状态更新消息到{}", to_uid);
    }
    else {
        warn!("{}用户没有连接", to_uid);
    }
    Ok(())
}

// 辅助函数：解析私聊消息类型
fn parse_message_type(content_type: &Option<String>) -> PrivateMsgType {
    content_type
        .as_ref()
        .and_then(|s| PrivateMsgType::from_enum_string(s))
        .unwrap_or(PrivateMsgType::Text)
}

// 发送私聊消息给在线用户
async fn send_private_message_online(
    payload: MesPayload,
    receiver_account: String,
    state: AppState,
) -> AppResult<()> {
    // 时间戳已经在调用方设置，不再使用 Utc::now()

    let mes_private = ClientMessage::Private(payload);
    let ws_mes_private = Message::Text(serde_json::to_string(&mes_private)
        .map_err(|e| AppError::SerializeFailure(e.to_string()))?
        .into());

    if let Some(tx) = state.connection_pool.get(&receiver_account) {
        tx.send(ws_mes_private)
            .map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
    }

    Ok(())
}

// 发送ACK的函数
async fn send_message_ack(
    sender_account: String,
    ack: MessageAck,
    state: AppState,
) -> AppResult<()> {
    // 构建ACK消息
    let ack_msg = ServerMessage::MessageAck(ack);
    let ws_msg = Message::Text(serde_json::to_string(&ack_msg)
        .map_err(|e| AppError::SerializeFailure(e.to_string()))?
        .into());

    // 发送给发送方
    if let Some(tx) = state.connection_pool.get(&sender_account) {
        tx.send(ws_msg)
            .map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
    }

    Ok(())
}

// 辅助函数：解析群聊消息类型
fn parse_group_message_type(content_type: &Option<String>) -> GroupMsgType {
    content_type
        .as_ref()
        .and_then(|s| GroupMsgType::from_enum_string(s))
        .unwrap_or(GroupMsgType::Text)
}

// 发送群聊消息广播
async fn send_group_message_broadcast(
    payload: MesPayload,
    state: AppState,
) -> AppResult<()> {
    // 时间戳已经在调用方设置，不再使用 Utc::now()

    let mes_group = ClientMessage::MesGroup(payload.clone());
    let group_id = payload.get_receiver_id()
        .ok_or_else(|| AppError::RecipientNotFound("群ID为空".to_string()))?;

    if let Some(channel) = state.broadcast_pool.get(group_id) {
        let tx = channel.tx.clone();
        tx.send(mes_group)
            .map_err(|e| AppError::BroadcastSenderFailure(e.to_string()))?;
    }

    Ok(())
}
