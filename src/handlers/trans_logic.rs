// // src/handlers/trans_logic.rs
// /* 
//     这个模块是用来处理前端通过websocket发来的不同类型的消息，例如私聊消息，群聊消息
//     以及后端要发送给前端的消息，例如广播群聊消息，好友上线消息
// */
// 库模块导入
use axum::extract::ws::{close_code, CloseFrame, Message};
use log::{info, warn};
use serde_json::json;
// 分离模块导入
use crate::models::errors::{AppError, AppResult};
use crate::models::msg_websocket::{ClientMessage, MesPayload, ServerMessage};
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
        warn!("{}账号没有连接", to_uid);
    }
    Ok(())
}