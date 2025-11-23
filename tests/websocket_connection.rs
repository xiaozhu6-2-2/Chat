// tests/websocket_connection.rs
mod common;

use common::TestContext;
use tokio_tungstenite::connect_async;
use tokio::time::Duration;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::tungstenite::Message;

#[tokio::test]
async fn test_websocket_connection_success() {
    let ctx = TestContext::new().await;
    
    // 先注册用户
    let register_data = json!({
        "account": "test_user_ws",
        "password": "test_password",
        "username": "WS Test User"
    });
    
    let response = ctx.client
        .post(&format!("{}/register", ctx.base_url))
        .json(&register_data)
        .send()
        .await
        .expect("Register request failed");
    
    assert!(response.status().is_success());
    
    // 登录获取token
    let token = ctx.login_user("test_user_ws", "test_password").await;
    
    // 连接WebSocket
    let ws_url = ctx.get_ws_url(&token).await;
    let (ws_stream, _) = connect_async(&ws_url).await.expect("Failed to connect");
    let (mut write, mut read) = ws_stream.split();
    
    // 测试连接是否成功建立
    tokio::time::timeout(Duration::from_secs(5), async {
        // 服务器应该发送欢迎消息或心跳
        let message = read.next().await.expect("No message received");
        assert!(matches!(message, Ok(_)));
    })
    .await
    .expect("Connection test timeout");
}

#[tokio::test]
async fn test_websocket_connection_invalid_token() {
    let ctx = TestContext::new().await;
    
    // 使用无效token连接
    let invalid_token = "invalid_token";
    let ws_url = ctx.get_ws_url(invalid_token).await;
    
    let result = connect_async(&ws_url).await;
    assert!(result.is_err(), "Should reject connection with invalid token");
}

#[tokio::test]
async fn test_websocket_connection_no_token() {
    let ctx = TestContext::new().await;
    
    // 没有token的连接应该被拒绝
    let ws_url = "ws://localhost:3001/auth/connection/ws";
    
    let result = connect_async(ws_url).await;
    assert!(result.is_err(), "Should reject connection without token");
}