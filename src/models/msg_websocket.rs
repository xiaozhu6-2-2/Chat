// src::models::msg_websocket.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
// 客户端发给服务端的消息结构
#[derive(Debug, Serialize, Deserialize)]
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
    // 关闭帧
    Close {

    }
}

// 服务端发给客户端的消息结构
#[derive(Debug, Serialize, Deserialize)]
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
    // 关闭帧
    Close {
        
    }
}