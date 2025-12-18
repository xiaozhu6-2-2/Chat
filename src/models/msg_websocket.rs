// src::models::msg_websocket.rs
use serde::{Deserialize, Serialize};
use serde_json::Value;
// 公共的消息数据负载
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MesPayload {
    // 消息元数据
    pub message_id: Option<String>,// 前端传来一个临时message_id用于ACK，真正的message_id由后端生成
    pub chat_id: Option<String>,// 前端传来，不改变
    timestamp: Option<i64>,// 后端生成

    // 发送者信息（前端写入）
    pub sender_id: Option<String>,
    sender_name: Option<String>,
    sender_avatar: Option<String>,

    // 接收者信息
    receiver_id: Option<String>,

    // 消息
    pub content_type: Option<String>,// 消息类型
    pub details: Option<String>,// 消息内容

    // 实时状态信息
    pub is_announcement: bool,// 是否是群聊公告
    pub mentioned_uids: Vec<String>,// @uid列表
    pub quote_msg_id: String,// 引用的信息
}
// 实现类方法
impl MesPayload {
    pub fn get_receiver_id(&self)-> Option<&String>{
        self.receiver_id.as_ref()
    }

    // 设置时间戳
    pub fn set_timestamp(&mut self, timestamp: Option<i64>) {
        self.timestamp = timestamp;
    }

    // 获取时间戳
    pub fn get_timestamp(&self) -> Option<i64> {
        self.timestamp
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

// ACK消息结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MessageAck {
    pub temp_message_id: String,  // 前端传来的临时ID
    pub message_id: String,       // 后端生成的正式ID
    pub timestamp: i64,           // 服务器时间戳
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
    },
    // 消息ACK确认
    MessageAck(MessageAck),
    // 消息错误
    MessageError {
        temp_message_id: String,
        error: String,
    }
}