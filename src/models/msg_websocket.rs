// src::models::msg_websocket.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
// 公共的消息数据负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MesPayload {
    messageId: Option<String>,
    timestamp: Option<i64>,
    senderId: Option<String>,
    receiverId: Option<String>,
    chatType: Option<String>,
    details: Option<String>,
}
// 实现类方法
impl MesPayload {
    pub fn get_receiverId(&self)-> Option<&String>{
        self.receiverId.as_ref()
    }
}

// 客户端发给服务端的消息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ClientMessage {
    // 心跳请求
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>
    },
    // 心跳响应
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>
    },
    // 群聊
    MesGroup (MesPayload),
    // 私聊 
    Private (MesPayload)
}

// 服务端发给客户端的消息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", content = "payload")]
pub enum ServerMessage {
// 心跳请求
    Ping {
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>
    },
    // 心跳响应
    Pong {
        #[serde(skip_serializing_if = "Option::is_none")]
        timestamp: Option<i64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>
    },
    // 在线状态更新
    UpdateOnlineState {
        uid: String,
        online_state: bool,
    }
}