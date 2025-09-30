use axum::extract::ws::Message;
// // src/handlers/trans_logic.rs
// /* 
//     这个模块是用来处理前端通过websocket发来的不同类型的消息，例如私聊消息，群聊消息
//     以及后端要发送给前端的消息，例如广播群聊消息，好友上线消息
// */
// 库模块导入
use axum::extract::State;
use log::{info, warn};
use serde_json::json;
// 分离模块导入
use crate::models::errors::{AppError, AppResult};
use crate::models::msg_websocket::{self, ClientMessage};
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
    let ws_pong = Message::Text(serde_json::to_string(&pong).unwrap().into());

    // 获取连接池
    let pool = state.connection_pool.read();

    // 找到对应的连接
    if let Some(tx) = pool.await.get(&account) {
        tx.send(ws_pong).map_err(|e| AppError::MpcsSenderFailure(e.to_string()))?;
        info!("成功发送Pong到{}", account);
    } else {
        warn!("{}账号没有连接", account);
    }
    
    Ok(())
}

// 发送close
pub async fn send_close(

) -> AppResult<()> {
    Ok(())
}

// 处理私聊消息
pub async fn handle_private_chat(

) -> AppResult<()> {
    Ok(())
}

// 处理群聊消息
pub async fn handle_group_chat(

) -> AppResult<()> {
    Ok(())
}