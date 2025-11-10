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
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, interval, Instant};
use futures::{stream::{SplitSink, SplitStream}, SinkExt, StreamExt};
use log::{info, error};
use serde_json::json;
// 模块分离导入
use crate::{
    handlers::trans_logic::{handle_group_chat, handle_private_chat, send_close, send_pong}, models::{
        errors::AppResult, msg_websocket::{ClientMessage, ServerMessage}, others::Claims
    }, state::{AppState}
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
    // 3、在mysql中查询用户已加入的所有群聊号
    // 4、将用户添加到redis的全局在线状态表，在redis中将用户添加至用户所在群聊的在线状态表
    // 5、在mysql中查询用户的好友账号，筛选在线好友，并将用户上线通知给这些在线好友的客户端
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
    // 2、清理全局在线状态表中用户在线状态信息
    // 3、清理群聊在线状态表中用户在线状态信息
    // 4、向用户的在线好友发送用户下线通知
*/
async fn handle_websocket(
    socket: WebSocket,
    claims: Claims,
    state: AppState
)-> () {
    // 连接者账号
    let account = claims.sub.clone();
    let account_for_send = account.clone();
    let account_for_recv = account.clone();
    let account_for_timeout = account.clone();

    // 状态
    let state_for_send = state.clone();
    let state_for_recv = state.clone();
    let state_for_timeout = state.clone();

    // 1、将WebSocket分为读写端
    let (sender, receiver) = socket.split();

    // 2、创建mpsc信道的读写端
    let (tx, rx) = mpsc::unbounded_channel();

    // 3、查询用户所在群聊

    // 4、更新用户在线状态

    // 5、通知好友

    // 将tx存入连接池,将账号和写端绑定
    state.connection_pool.insert(account.clone(), tx);
    info!("{}连接成功", account);

    // 记录最后一次心跳的时间，用于超时判断
    let last_activity = Arc::new(tokio::sync::RwLock::new(Instant::now())); 
    // 克隆智能指针，让不同的任务共享一块内存
    let last_activity_for_send = Arc::clone(&last_activity);
    let last_activity_for_recv = Arc::clone(&last_activity);
    let last_activity_for_timeout = Arc::clone(&last_activity);

    // WebSocket写任务(监听专用信道，并向 WebSocket 发送消息)
    let send_task = tokio::spawn(async move {
        send_task_spawn(rx, sender).await
    });

    // WebSocket读任务(监听 WebSocket 接收消息)
    let recv_tack = tokio::spawn(async move{
        recv_tack_spawn(receiver, last_activity_for_recv, state_for_recv, account_for_recv).await
    });

    // 超时机制
    let timeout_task = tokio::spawn(async move{
        timeout_task_spawn(last_activity_for_timeout, account_for_timeout, state_for_timeout).await
    });

    // 结束连接:当读任务或者写任务任意一个结束时，结束连接
    tokio::select! {
        _ = send_task => {},
        _ = recv_tack => {},
        _ = timeout_task => {}
    }

    // 从连接池清除该连接
    state.connection_pool.remove(&claims.sub);

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
async fn recv_tack_spawn(
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
                    Ok(ClientMessage::Pong { timestamp, data }) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                    },
                    // 心跳请求消息
                    Ok(ClientMessage::Ping { timestamp, data }) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 回复pong(注：这里需要错误处理)
                        let _ = send_pong(account.clone(), state.clone()).await;
                    },
                    // 私聊消息
                    Ok(ClientMessage::Private (payload )) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 处理私聊消息(注：这里需要错误处理)
                        let _ = handle_private_chat(payload, state.clone()).await;
                    },
                    // 群聊消息
                    Ok(ClientMessage::MesGroup (payload)) => {
                        // 更新时间
                        let mut last_activity = last_activity_for_recv.write().await;
                        *last_activity = Instant::now();
                        // 处理群聊消息(注：这里需要错误处理)
                        let _ = handle_group_chat(payload, state.clone()).await;
                    },
                    _ => {

                    }
                }
            },
            // 关闭帧
            Message::Close(msg) => {
                // 发送close帧(注：这里需要错误处理)
                let _ = send_close(account.clone(), state.clone()).await;
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
            // 发送close帧(注：这里需要错误处理)
            let _ = send_close(account.clone(), state.clone()).await;
            break;
        }
    }
}