// tests/websocket_heartbeat.rs
mod common;

use common::TestContext;
use futures::StreamExt;
use serde_json::{json, Value};
use tokio::time::{timeout, Duration};
use futures_util::SinkExt;
use tokio_tungstenite::connect_async;

#[tokio::test]
async fn test_heartbeat_ping_pong() {
    let ctx = TestContext::new().await;
    
    // 注册并登录用户
    let register_data = json!({
        "account": "heartbeat_user",
        "password": "password",
        "username": "Heartbeat Test"
    });
    
    ctx.client
        .post(&format!("{}/register", ctx.base_url))
        .json(&register_data)
        .send()
        .await
        .expect("Register failed");
    
    let token = ctx.login_user("heartbeat_user", "password").await;
    let ws_url = ctx.get_ws_url(&token).await;
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();
    
    // 测试服务器发送的Ping和客户端的Pong响应
    timeout(Duration::from_secs(35), async move {
        while let Some(message) = read.next().await {
            if let Ok(tokio_tungstenite::tungstenite::Message::Text(text)) = message {
                if let Ok(payload) = serde_json::from_str::<Value>(&text) {
                    if payload["type"] == "Ping" {
                        // 发送Pong响应
                        let pong = json!({
                            "type": "Pong",
                            "timestamp": chrono::Utc::now().timestamp(),
                            "data": {"source": "client"}
                        });
                        
                        write.send(tokio_tungstenite::tungstenite::Message::Text(pong.to_string().into())).await.expect("Failed to send pong");
                        break;
                    }
                }
            }
        }
    })
    .await
    .expect("Heartbeat test timeout");
}

#[tokio::test]
async fn test_heartbeat_timeout() {
    let ctx = TestContext::new().await;
    
    // 注册并登录用户
    let register_data = json!({
        "account": "timeout_user",
        "password": "password", 
        "username": "Timeout Test"
    });
    
    ctx.client
        .post(&format!("{}/register", ctx.base_url))
        .json(&register_data)
        .send()
        .await
        .expect("Register failed");
    
    let token = ctx.login_user("timeout_user", "password").await;
    let ws_url = ctx.get_ws_url(&token).await;
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (_, mut read) = ws_stream.split();
    
    // 不响应心跳，等待超时断开
    let start_time = tokio::time::Instant::now();
    
    while let Some(message) = read.next().await {
        if let Ok(tokio_tungstenite::tungstenite::Message::Close(_)) = message {
            break;
        }
    }
    
    let elapsed = start_time.elapsed();
    // 应该在90秒左右超时
    assert!(elapsed >= Duration::from_secs(85) && elapsed <= Duration::from_secs(95));
}