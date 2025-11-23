// tests/websocket_messages.rs
mod common;

use common::TestContext;
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};
use tokio_tungstenite::tungstenite::{Message};
use tokio_tungstenite::connect_async;
#[tokio::test]
async fn test_private_message_sending() {
    let ctx = TestContext::new().await;
    
    // 注册两个用户
    for i in 0..2 {
        let register_data = json!({
            "account": format!("private_user_{}", i),
            "password": "password",
            "username": format!("Private User {}", i)
        });
        
        ctx.client
            .post(&format!("{}/register", ctx.base_url))
            .json(&register_data)
            .send()
            .await
            .expect("Register failed");
    }
    
    let token1 = ctx.login_user("private_user_0", "password").await;
    let token2 = ctx.login_user("private_user_1", "password").await;
    
    // 两个用户都连接WebSocket
    let ws_url1 = ctx.get_ws_url(&token1).await;
    let ws_url2 = ctx.get_ws_url(&token2).await;
    
    let (ws_stream1, _) = connect_async(&ws_url1).await.expect("User 1 connect failed");
    let (mut write1, mut read1) = ws_stream1.split();
    
    let (ws_stream2, _) = connect_async(&ws_url2).await.expect("User 2 connect failed");
    let (mut write2, mut read2) = ws_stream2.split();
    
    // 用户1发送私聊消息给用户2
    let private_message = json!({
        "type": "Private",
        "payload": {
            "messageId": "test_msg_1",
            "timestamp": chrono::Utc::now().timestamp(),
            "senderId": "private_user_0",
            "receiverId": "private_user_1",
            "chatType": "private",
            "details": "Hello from user 1!"
        }
    });
    
    write1.send(Message::Text(private_message.to_string())).await.expect("Send message failed");
    
    // 用户2应该收到消息
    timeout(Duration::from_secs(5), async move {
        while let Some(message) = read2.next().await {
            if let Ok(Message::Text(text)) = message {
                if let Ok(payload) = serde_json::from_str::<Value>(&text) {
                    if payload["type"] == "Private" {
                        let message_payload = &payload["payload"];
                        assert_eq!(message_payload["senderId"], "private_user_0");
                        assert_eq!(message_payload["details"], "Hello from user 1!");
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("Private message test timeout");
}

#[tokio::test]
async fn test_group_message_broadcast() {
    let ctx = TestContext::new().await;
    
    // 这个测试需要先实现群聊功能
    // 暂时跳过，等群聊功能实现后再测试
}