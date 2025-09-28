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
use tokio::sync::mpsc;
use tokio::time::{Duration, interval, Instant};
use futures::{SinkExt, StreamExt};
use log::info;
use serde_json::json;
// 模块分离导入
use crate::{
    models::{
        errors::AppResult,
        others::Claims,
        msg_websocket::ServerMessage,
        msg_websocket::ClientMessage,
    },
    state::AppState
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

// 用于处理WebSocket通信
async fn handle_websocket(
    mut socket: WebSocket,
    claims: Claims,
    state: AppState
) {
    // 连接者账号
    let account = claims.sub.clone();

    info!("{}连接成功", account);

    // 将WebSocket分为读写端
    let (mut sender, mut receiver) = socket.split();

    // 创建mpsc信道的读写端
    let (tx, mut rx) = mpsc::unbounded_channel();

    // 
    // 将tx存入连接池(花括号是为了释放锁)
    {
        let mut pool = state.connection_pool.write().await;
        // 将账号和写端绑定
        pool.insert(claims.sub.clone(), tx);
    }

    // 记录最后一次心跳的时间，用于超时判断
    let last_activity = Arc::new(tokio::sync::RwLock::new(Instant::now())); 
    // 克隆智能指针，让不同的任务共享一块内存
    let last_activity_for_send = Arc::clone(&last_activity);
    let last_activity_for_recv = Arc::clone(&last_activity);
    let last_activity_for_timeout = Arc::clone(&last_activity);

    // WebSocket写任务(监听专用信道，并向 WebSocket 发送消息)
    let send_task = tokio::spawn(async move{
        // 心跳计时器，每30s发送一次ping
        let mut heartbeat_interval = interval(Duration::from_secs(30));

        loop {
            tokio::select! {
                // 从mpsc信道收到发送任务，执行发送任务
                // 如果是close帧，发送后立刻跳出循环...
                msg = rx.recv() => {
                    if let Some(msg) = msg {
                        if sender.send(msg).await.is_err() {
                            // 发送失败就断开连接
                            break;
                        }
                    }
                    else{
                        // mpsc信道关闭，退出循环
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
                        break;
                    }
                }
            }
        }
    });

    // WebSocket读任务(监听 WebSocket 接收消息)
    let recv_tack = tokio::spawn(async move{
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
                            // 回复ping的任务...
                        },
                        _ => {

                        }
                    }
                },
                // 关闭帧
                Message::Close(_) => {
                    // 发送close帧...
                    // 关闭连接
                    break;
                },
                _ => {

                }
            }
        }
    });

    // 超时机制
    let timeout_task = tokio::spawn(async move{
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
                info!("连接{}心跳超时", account);
                // 发送close帧...
                break;
            }
        }
    });

    // 结束连接:当读任务或者写任务任意一个结束时，结束连接
    tokio::select! {
        _ = send_task => {},
        _ = recv_tack => {},
        _ = timeout_task => {}
    }

    // 从连接池清除该连接
    let mut pool = state.connection_pool.write().await;
    pool.remove(&claims.sub);

    // 日志
    info!("用户{}断开连接", claims.sub);
}