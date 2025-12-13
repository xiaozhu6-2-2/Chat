use std::sync::Arc;

// // src/handlers/trans_logic.rs
// /*
//     这个模块是用来处理前端通过websocket发来的不同类型的消息，例如私聊消息，群聊消息
//     以及后端要发送给前端的消息，例如广播群聊消息，好友上线消息
// */
// 库模块导入
use axum::extract::ws::{close_code, CloseFrame, Message};
use chrono::Utc;
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
use crate::models::entities::{PrivateMessage, PrivateMsgType, GroupMessage, GroupMsgType};
use crate::models::others::GroupBroadcastChannel;
use crate::models::repository::{UserRepository, FriendshipRepository, GroupChatRepository, PrivateChatRepository, GroupMessageRepository};
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

    // 2. 获取接收者account（注意：get_receiver_id返回的是account，不是uid）
    let receiver_account = payload.get_receiver_id()
        .ok_or_else(|| AppError::RecipientNotFound("接收者account为空".to_string()))?
        .clone();

    // 3. 通过account获取接收者uid
    let receiver_user = state.db_pool.find_user_by_account(&receiver_account).await?;
    let receiver_id = receiver_user.uid;

    // 4. 验证权限
    state.db_pool.validate_private_message_permission(&sender_uid, &receiver_id).await?;

    // 5. 生成消息ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let message_id = snowflake.next_id()?.to_string();

    // 6. 获取私聊会话（根据好友关系，会话必然存在）
    let private_chat = state.db_pool
        .find_chat_by_users(&sender_uid, &receiver_id)
        .await?;

    let chat_id = match private_chat {
        Some(chat) => chat.pid,
        None => {
            return Err(AppError::NotFound("私聊会话不存在".to_string()));
        }
    };

    // 7. 构建消息实体（send_time设为None，让数据库使用默认值）
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

    // 8. 保存消息到数据库
    PrivateChatRepository::save_message(&state.db_pool, message).await?;

    // 9. 从数据库获取刚刚保存的消息（获取数据库生成的时间戳）
    let saved_message = PrivateChatRepository::find_message_by_id(&state.db_pool, &message_id).await?
        .ok_or_else(|| AppError::NotFound("消息保存失败".to_string()))?;

    // 10. 获取数据库生成的时间戳
    let timestamp = saved_message.send_time
        .ok_or_else(|| AppError::NotFound("消息时间戳缺失".to_string()))?
        .timestamp();

    // 11. 获取发送者account（用于发送ACK）
    let sender_account = state.db_pool
        .find_user_by_uid(&sender_uid).await?
        .account;

    // 12. 检查接收者是否在线（直接检查WebSocket连接池）
    let is_receiver_online = state.connection_pool.contains_key(&receiver_account);

    // 13. 发送消息（在线）或保存离线消息
    if is_receiver_online {
        // 在线 - 直接发送
        send_private_message_online(payload.clone(), receiver_account, state.clone()).await?;
    }
    // 离线 - 消息已保存到数据库，无需额外操作

    // 14. 发送ACK给发送方
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

    // 2. 获取群ID
    let group_id = payload.get_receiver_id()
        .ok_or_else(|| AppError::RecipientNotFound("群ID为空".to_string()))?
        .clone();

    // 3. 验证群成员权限
    state.db_pool.validate_group_message_permission(&sender_uid, &group_id).await?;

    // 4. 生成消息ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let message_id = snowflake.next_id()?.to_string();

    // 5. 构建群聊消息实体（send_time设为None，让数据库使用默认值）
    // 先克隆需要使用的字段
    let content = payload.details.clone().unwrap_or_default();
    let content_type = payload.content_type.clone();
    let mentioned_uids = payload.mentioned_uids.clone();
    let quote_msg_id = payload.quote_msg_id.clone();
    let is_announcement = payload.is_announcement;
    let temp_message_id = payload.message_id.clone().unwrap_or_default();

    let message = crate::models::entities::GroupMessage {
        msg_id: message_id.clone(),
        gid: group_id.clone(),
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

    // 6. 保存消息到数据库
    GroupMessageRepository::save_message(&state.db_pool, message).await?;

    // 7. 从数据库获取刚刚保存的消息（获取数据库生成的时间戳）
    let saved_message = GroupMessageRepository::find_message_by_id(&state.db_pool, &message_id).await?
        .ok_or_else(|| AppError::NotFound("消息保存失败".to_string()))?;

    // 8. 获取数据库生成的时间戳
    let timestamp = saved_message.send_time
        .ok_or_else(|| AppError::NotFound("消息时间戳缺失".to_string()))?
        .timestamp();

    // 9. 获取发送者account
    let sender_account = state.db_pool
        .find_user_by_uid(&sender_uid).await?
        .account;

    // 10. 广播消息到群聊频道
    send_group_message_broadcast(payload, state.clone()).await?;

    // 11. 发送ACK给发送方
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
    match content_type.as_deref() {
        Some("text") => PrivateMsgType::Text,
        Some("image") => PrivateMsgType::Image,
        Some("file") => PrivateMsgType::File,
        Some("voice") => PrivateMsgType::Voice,
        Some("video") => PrivateMsgType::Video,
        Some("link") => PrivateMsgType::Link,
        Some("emoji") => PrivateMsgType::Emoji,
        Some("annoucement") => PrivateMsgType::Annoucement,
        _ => PrivateMsgType::Text,
    }
}

// 发送私聊消息给在线用户
async fn send_private_message_online(
    mut payload: MesPayload,
    receiver_account: String,
    state: AppState,
) -> AppResult<()> {
    // 更新payload中的时间戳
    payload.set_timestamp(Some(Utc::now().timestamp()));

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
    match content_type.as_deref() {
        Some("text") => GroupMsgType::Text,
        Some("image") => GroupMsgType::Image,
        Some("file") => GroupMsgType::File,
        Some("voice") => GroupMsgType::Voice,
        Some("video") => GroupMsgType::Video,
        Some("link") => GroupMsgType::Link,
        Some("emoji") => GroupMsgType::Emoji,
        Some("annoucement") => GroupMsgType::Annoucement,
        _ => GroupMsgType::Text,
    }
}

// 发送群聊消息广播
async fn send_group_message_broadcast(
    mut payload: MesPayload,
    state: AppState,
) -> AppResult<()> {
    // 更新payload中的时间戳
    payload.set_timestamp(Some(Utc::now().timestamp()));

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