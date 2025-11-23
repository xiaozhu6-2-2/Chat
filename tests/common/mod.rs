// tests/common/mod.rs
use echat::{create_test_app, Application};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;
use tokio_tungstenite::{connect_async, tungstenite::Message as WsMessage};
use futures_util::{SinkExt, StreamExt};

pub struct TestContext {
    pub app: Application,
    pub base_url: String,
    pub client: Client,
}

impl TestContext {
    pub async fn new() -> Self {
        let app = create_test_app().await.expect("Failed to create test app");
        let port = 3001; // 使用固定端口避免冲突
        let base_url = format!("http://localhost:{}", port);
        
        // 在后台启动服务器
        let app_clone = app.clone();
        tokio::spawn(async move {
            app_clone.run().await.expect("Server failed");
        });
        
        // 等待服务器启动
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        Self {
            app,
            base_url,
            client: Client::new(),
        }
    }
    
    pub async fn get_ws_url(&self, token: &str) -> String {
        format!("ws://localhost:3001/auth/connection/ws?token={}", token)
    }
    
    pub async fn login_user(&self, account: &str, password: &str) -> String {
        let login_data = json!({
            "account": account,
            "password": password
        });
        
        let response = self.client
            .post(&format!("{}/login", self.base_url))
            .json(&login_data)
            .send()
            .await
            .expect("Login request failed");
        
        assert!(response.status().is_success());
        
        let login_response: serde_json::Value = response.json().await.expect("Failed to parse login response");
        login_response["token"].as_str().expect("No token in response").to_string()
    }
}

