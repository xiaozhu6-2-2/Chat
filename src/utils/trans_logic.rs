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
use crate::models::msg_websocket::{ClientMessage, MesPayload, ServerMessage};
use crate::models::others::GroupBroadcastChannel;
use crate::models::repository::UserRepository;
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

// 处理私聊消息
pub async fn handle_private_chat(
    payload: MesPayload,
    state: AppState
) -> AppResult<()> {
    // 构建私聊消息
    let mes_private = ClientMessage::Private(payload.clone());

    // 构建WebSocket文本消息
    let ws_mes_private = Message::Text(serde_json::to_string(&mes_private)
        .map_err(|e| 
            AppError::SerializeFailure(e.to_string())
        )?
        .into()
    );

    // 接收人账号
    let account = payload.get_receiverId().ok_or_else(|| AppError::RecipientNotFound("接收者为空".to_string()))?;

    // 获取tx
    let tx = state.connection_pool.get(account).map(|guard| {
        guard.value().clone()// 克隆tx并释放锁
    });

    // 找到对应连接并发送
    if let Some(tx) = tx{
        tx.send(ws_mes_private).map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
        info!("成功发送私聊消息到{}", account);
    } else {
        warn!("{}账号没有连接", account);
    }

    Ok(())
}

// 处理群聊消息
pub async fn handle_group_chat(
    payload: MesPayload,
    state: AppState
) -> AppResult<()> {
    // 构建群聊信息
    let mes_group = ClientMessage::MesGroup(payload.clone());

    // 群号
    let group_id = payload.get_receiverId().ok_or_else(|| AppError::RecipientNotFound("群号为空".to_string()))?;

    // 获取broadcast频道
    let channel = state.broadcast_pool.get(group_id).map(|guard| {
        guard.value().clone()//克隆频道并释放锁
    });

    // 取出频道发送端
    if let Some(channel) = channel{
        let tx = channel.tx.clone();
        tx.send(mes_group).map_err(|e| AppError::BroadcastSenderFailure(e.to_string()))?;
        info!("成功发送群聊消息到群聊频道{}", group_id);
    } else {
        warn!("{}频道不存在", group_id);
    }
    
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

    // 订阅群聊频道
    let mut rx = channel.tx.subscribe();
    
    // 增加订阅者计数
    let old_count = channel.subscriber_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    info!("用户 {} 开始监听群聊 {} 频道，当前订阅者数量: {}", account, gid, old_count + 1);
    
    // 减少计数
    let account_for_guard = account.clone();
    let gid_for_guard = gid.clone();
    let _guard = guard((), move |_| {
        if let Some(channel) = broadcast_pool.get(&gid_for_guard) {
            let count = channel.subscriber_count.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
            info!("用户 {} 退出群聊 {} 频道，剩余订阅者: {}", account_for_guard, gid_for_guard, count - 1);

            // 清理频道（最后一个订阅者）
            if count <= 1 {
                info!("清理空闲群聊频道: {}", gid_for_guard);
                broadcast_pool.remove(&gid_for_guard);
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