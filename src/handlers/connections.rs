use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade}, response::Response, Extension
};
use log::info;
use crate::{
    models::{errors::AppResult, others::Claims},
    state::AppState
};

// src/handlers/connections.rs
/*
这个模块是用来处理用户发来的websocket请求以及实现用户上线逻辑
*/ 
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>
)-> AppResult<Response> {
    info!("用户{}正在建立连接", claims.sub);
    
    // 将http升级到websocket，并且调用处理函数，将claims和state都传递下去
    Ok(ws.on_upgrade(move |socket|handle_websocket(socket, claims, state)))
}

async fn handle_websocket(
    mut socket: WebSocket,
    claims: Claims,
    state: AppState
) {

}