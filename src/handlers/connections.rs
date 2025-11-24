// src/handlers/connections.rs
/*
这个模块是用来处理用户发来的websocket请求以及实现用户上线逻辑
*/ 
// 库模块导入
use axum::{
    extract::{ws::WebSocket, ws::Message, State, WebSocketUpgrade},
    response::Response,
    Extension,
};
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast::{self}, mpsc::{self, UnboundedSender}};
use tokio_util::sync::CancellationToken;
use tokio::time::{Duration, interval, Instant};
use futures::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use log::{error, info, warn};
use scopeguard::guard;
use serde_json::json;
// 模块分离导入
use crate::{
    handlers::trans_logic::{handle_group_chat, handle_private_chat, send_close, send_online_state, send_pong}, models::{
        entities::UserOnline, errors::AppResult, msg_websocket::{ClientMessage, ServerMessage}, others::{Claims, GroupBroadcastChannel}, repository::{FriendshipRepository, GroupChatRepository, OnlineRepository, UserRepository}
    }, repository::OnlineRepository::OnlineManager, state::AppState
};

// 用于建立WebSocket连接
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>
)-> AppResult<Response> {
    info!("用户{}正在建立连接", claims.sub);
    
    // 将http升级到websocket，并且调用处理函数，将claims和state都传递下去
    Ok(ws.on_upgrade(move |socket| handle_websocket(socket, claims, state)))
}

// 用于处理WebSocket通信(错误不再继续传播)
/*
    ===============预处理===================
    1、将WebSocket连接分为读写端
    2、创建MPSC信道并分为读写端，将写端存入连接池，与用户账号标识
    3、在mysql中查询用户已加入的所有群聊号
    4、将用户添加到redis的全局在线状态表，在redis中将用户添加至用户所在群聊的在线状态表
    5、连接所在群聊频道
    // 6、在mysql中查询用户的好友账号，筛选在线好友，并将用户上线通知给这些在线好友的客户端
    ===============三个任务==================
    1、需要克隆并传递给三个任务的变量：用户账号account、应用状态state、最后一次心跳的时间last_activity
    2、写任务函数：
        ⅰ)监听MPSC读端，并将接收到的信息发送到客户端
        ⅱ)定时发送心跳请求消息(Ping)
    3、读任务函数：
        负责从WebSocket连接中接收到的各类消息，并根据消息类型进行处理动作
    4、超时任务函数：
        通过last_activity进行超时判断
    ===============断连清理==================
    1、清理连接池中的MPSC信道
    2、清理在线状态
    3、退出所在群聊频道 or 如果是最后一个用户则清理所在群聊频道
    4、向用户的在线好友发送用户下线通知
*/
async fn handle_websocket(
    socket: WebSocket,
    claims: Claims,
    state: AppState
)-> () {
    // 连接者账号
    let account = claims.sub.clone();
    let account_for_recv = account.clone();
    let account_for_timeout = account.clone();

    // 状态
    let state_for_recv = state.clone();
    let state_for_timeout = state.clone();

    // 1、将WebSocket分为读写端
    let (sender, receiver) = socket.split();

    // 2、创建mpsc信道的读写端
    let (tx, rx) = mpsc::unbounded_channel();

    // 3、查询用户所在群聊
    // 查找uid
    let user = match state.db_pool.find_user_by_account(&account).await {
        Ok(user) => user,
        Err(e) => {
            error!("查找用户失败: {}", e);
            return;
        }
    };

    // 根据uid查找群聊
    let records = match state.db_pool.find_groups_by_user(&user.uid).await {
        Ok(records) => records,
        Err(e) => {
            error!("查找用户群聊失败: {}", e);
            return;
        }
    };

    // 4、更新用户在线状态
    // 将record转换为gid
    let group_ids : Vec<String> = records.into_iter().map(|record| record.gid).collect();

    // 更新在线状态
    if let Err(e) = OnlineManager::user_online(
        &state.redis_pool, 
        UserOnline {
            account : user.account,
            username : user.username
        },
        &group_ids).await 
    {
        error!("用户上线失败: {}", e);
        return;
    }

    // 5、连接所在群聊频道
    // 创建取消执行令牌
    let cancel_token = CancellationToken::new();

    // 创建监听任务列表
    let mut listen_handlers = Vec::new();

    // 创建任务监听每个group_id
    for group_id in &group_ids {
        // 克隆需要转移所有权的变量
        let account_for_listen = account.clone();
        let gid = group_id.clone();
        let tx_for_listen = tx.clone();// mpsc发送信道
        let broadcast_pool_for_listen = state.broadcast_pool.clone();
        let child_token = cancel_token.child_token();// 派生子令牌

        // 创建监听任务
        let listen = tokio::spawn(async move {
            group_channel_listen(gid, account_for_listen, tx_for_listen, broadcast_pool_for_listen, child_token).await
        });

        // 加入到监听任务列表
        listen_handlers.push(listen);
    }

    // 6、通知好友
    // 查找好友记录
    let records = match state.db_pool.find_friendship_by_uid(&user.uid).await {
        Ok(records) => records,
        Err(e) => {
            error!("查找用户好友失败: {}",e);
            return;
        }
    };
    
    
    let uid_for_friend = user.uid.clone();
    // 提取好友id
    let friend_uids: Vec<String> = records.into_iter().map(|record|
        if uid_for_friend == record.uid {
            record.to_uid
        }
        else {
            record.uid
        }
    ).collect();

    let online_state_msg = ServerMessage::UpdateOnlineState { uid: user.uid.clone(), online_state: true };
    // 发送上线通知
    for friend_uid in &friend_uids {
        match send_online_state(friend_uid.clone(), online_state_msg.clone(), state.clone()).await {
            Ok(()) => {},
            Err(e) => {
                error!("发送上线通知失败: {}", e);
                return;
            }
        }
    }

    // 将tx存入连接池,将账号和写端绑定
    state.connection_pool.insert(account.clone(), tx);
    info!("{}连接成功", account);

    // 记录最后一次心跳的时间，用于超时判断
    let last_activity = Arc::new(tokio::sync::RwLock::new(Instant::now()));
    // 克隆智能指针，让不同的任务共享一块内存
    let last_activity_for_recv = Arc::clone(&last_activity);
    let last_activity_for_timeout = Arc::clone(&last_activity);

    // WebSocket写任务(监听专用信道，并向 WebSocket 发送消息)
    let send_task = tokio::spawn(async move {
        send_task_spawn(rx, sender).await
    });

    // WebSocket读任务(监听 WebSocket 接收消息)
    let recv_task = tokio::spawn(async move{
        recv_task_spawn(receiver, last_activity_for_recv, state_for_recv, account_for_recv).await
    });

    // 超时机制
    let timeout_task = tokio::spawn(async move{
        timeout_task_spawn(last_activity_for_timeout, account_for_timeout, state_for_timeout).await
    });

    // 结束连接:当读任务或者写任务任意一个结束时，结束连接
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
        _ = timeout_task => {}
    }

    // 1、从连接池清除该连接
    state.connection_pool.remove(&account);

    // 2、下线
    if let Err(e) = OnlineManager::user_offline(&state.redis_pool, account.clone(), &group_ids).await {
        error!("用户下线失败{}", e);
    }

    // 3、清理群聊频道
    cancel_token.cancel();
    for handle in listen_handlers {
        match tokio::time::timeout(Duration::from_millis(100), handle).await {
            Ok(_) => {
                info!("群聊监听任务优雅退出");
            },
            Err(_) => {
                warn!("群聊监听任务退出超时，强制终止");
            }
        }
    }

    // 4、向好友发送下线通知
    let offline_state_msg = ServerMessage::UpdateOnlineState { uid: user.uid, online_state: false };
    for friend_uid in &friend_uids {
        match send_online_state(friend_uid.clone(), offline_state_msg.clone(), state.clone()).await {
            Ok(()) => {},
            Err(e) => {
                error!("发送下线通知失败: {}", e);
            }
        }
    }

    // 日志
    info!("用户{}断开连接", claims.sub);
}

// WebSocket发送任务
async fn send_task_spawn(
    mut rx : mpsc::UnboundedReceiver<Message>,
    mut sender: SplitSink<WebSocket, Message>,
) {
    // 心跳计时器，每30s发送一次ping
    let mut heartbeat_interval = interval(Duration::from_secs(30));

    loop {
        tokio::select! {
            // 从mpsc信道收到发送任务，执行发送任务
            // 如果是close帧，发送后立刻跳出循环...
            msg = rx.recv() => {
                if let Some(msg) = msg {
                    if sender.send(msg).await.is_err() {
                        // 发送失败就断开连接，并记录日志
                        error!("WebSocket发送信息失败");
                        break;
                    }
                }
                else{
                    // mpsc信道关闭，退出循环
                    error!("mpsc信道关闭");
                    break;
                }
            },
            _ = heartbeat_interval.tick() => {
                // 创建自定义的ping消息
                let ping_msg = ServerMessage::Ping{
                    timestamp: Some(chrono::Utc::now().timestamp()),
                    data: Some(json!({"source": "server"}))
                };

                // 转换为JSON字符串，再包装为WebSocket Message
                let ws_msg = Message::Text(serde_json::to_string(&ping_msg).unwrap().into());

                if sender.send(ws_msg).await.is_err() {
                    // 发送失败就断开连接
                    error!("ping消息发送失败");
                    break;
                }
            }
        }
    }
}

// WebSocket接收任务
async fn recv_task_spawn(
    mut receiver: SplitStream<WebSocket>,
    last_activity_for_recv: Arc<RwLock<Instant>>,
    state: AppState,
    account: String
) {
    // 从websocket中获取消息
    while let Some(Ok(msg)) = receiver.next().await {
        // 根据帧类型处理消息
        match msg {
            // 文本消息
            Message::Text(text) => {
                match serde_json::from_str::<ClientMessage>(&text) {
                    // 心跳响应消息
                    Ok(ClientMessage::Pong { timestamp: _, data: _ }) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                    },
                    // 心跳请求消息
                    Ok(ClientMessage::Ping { timestamp: _, data: _ }) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 回复pong
                        if let Err(e) = send_pong(account.clone(), state.clone()).await {
                            error!("回复pong错误 {}", e);
                        }
                    },
                    // 私聊消息
                    Ok(ClientMessage::Private (payload )) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 处理私聊消息
                        if let Err(e) = handle_private_chat(payload, state.clone()).await {
                            error!("处理私聊消息错误 {}", e);
                        }
                    },
                    // 群聊消息
                    Ok(ClientMessage::MesGroup (payload)) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 处理群聊消息
                        if let Err(e) = handle_group_chat(payload, state.clone()).await {
                            error!("处理群聊消息错误 {}", e);
                        }
                    },
                    _ => {

                    }
                }
            },
            // 关闭帧
            Message::Close(msg) => {
                // 发送close帧
                if let Err(e) = send_close(account.clone(), state.clone()).await {
                    error!("发送关闭帧失败 {}", e);
                }
                // 关闭连接
                info!("客户端发来关闭帧:{:?}，关闭连接", msg);
                break;
            },
            _ => {

            }
        }
    }
}

// WebSocket超时任务
async fn timeout_task_spawn(
    last_activity_for_timeout: Arc<RwLock<Instant>>,
    account: String,
    state: AppState
) {
    // 10秒检测一次
    let mut check_interval = interval(Duration::from_secs(10));

    loop {
        // 等待定时器触发
        check_interval.tick().await;

        // 获取读者锁
        let last_activity = last_activity_for_timeout.read().await;

        // 超过90秒无活动
        if last_activity.elapsed() > Duration::from_secs(90) {
            // 自动断开连接
            error!("连接{}心跳超时", account);
            // 发送close帧
            if let Err(e) = send_close(account.clone(), state.clone()).await {
                error!("发送关闭帧失败 {}", e);
            }
            break;
        }
    }
}

// 群聊监听任务
async fn group_channel_listen(
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
        tokio::select! {
            result = rx.recv() => {
                match result {
                    Ok(msg) => {
                        let msg_json = match serde_json::to_string(&msg) {
                            Ok(msg) => msg,
                            Err(e) => {
                                error!("序列化失败: {}", e);
                                break;
                            }
                        };
                        
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