// src/handlers/connections.rs
/*
这个模块是用来处理用户发来的websocket请求以及实现用户上线逻辑
*/ 
// 库模块导入
use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade}, response::Response, Extension
};
use tokio::sync::mpsc;
use futures::{SinkExt, StreamExt};
use log::info;
// 模块分离导入
use crate::{
    models::{errors::AppResult, others::Claims},
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
    // 将WebSocket分为读写端
    let (mut sender, mut receiver) = socket.split();

    // 创建mpsc信道的读写端
    let (tx, mut rx) = mpsc::unbounded_channel();

    // 将tx存入连接池(花括号是为了释放锁)
    {
        let mut pool = state.connection_pool.write().await;
        // 将账号和写端绑定
        pool.insert(claims.sub.clone(), tx);
    }

    // WebSocket写任务(监听专用信道，并向 WebSocket 发送消息)
    let send_task = tokio::spawn(async move{
        // 从mpsc信道收到发送任务，执行发送任务
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // WebSocket读任务(监听 WebSocket 接收消息)
    let recv_tack = tokio::spawn(async move{
        while let Some(Ok(msg)) = receiver.next().await {
            // 根据类型处理消息
        }
    });

    // 结束连接:当读任务或者写任务任意一个结束时，结束连接
    tokio::select! {
        _ = send_task => {},
        _ = recv_tack => {}
    }

    // 从连接池清除该连接
    let mut pool = state.connection_pool.write().await;
    pool.remove(&claims.sub);
}