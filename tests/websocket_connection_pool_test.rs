// // tests/websocket_connection_pool.rs
// mod common;

// use common::TestContext;
// use serde_json::json;
// use tokio_tungstenite::connect_async;
// use tokio::time::Duration;
// #[tokio::test]
// async fn test_connection_pool_management() {
//     let ctx = TestContext::new().await;
    
//     // 注册多个用户
//     for i in 0..3 {
//         let register_data = json!({
//             "account": format!("pool_user_{}", i),
//             "password": "password",
//             "username": format!("Pool User {}", i)
//         });
        
//         ctx.client
//             .post(&format!("{}/register", ctx.base_url))
//             .json(&register_data)
//             .send()
//             .await
//             .expect("Register failed");
//     }
    
//     let mut connections = Vec::new();
    
//     // 建立多个连接
//     for i in 0..3 {
//         let token = ctx.login_user(&format!("pool_user_{}", i), "password").await;
//         let ws_url = ctx.get_ws_url(&token).await;
        
//         let (ws_stream, _) = connect_async(&ws_url).await.expect(&format!("User {} connect failed", i));
//         connections.push(ws_stream);
//     }
    
//     // 验证连接池中有3个连接
//     // 注意：这里需要为测试暴露连接池的访问方法
//     // 可以在AppState中添加测试专用的方法
//     let connection_count = ctx.app.router().state().connection_pool.len();
//     assert_eq!(connection_count, 3);
    
//     // 断开一个连接
//     drop(connections.pop());
    
//     // 等待连接清理
//     tokio::time::sleep(Duration::from_secs(1)).await;
    
//     // 验证连接池中减少了一个连接
//     let connection_count = ctx.app.router().state().connection_pool.len();
//     assert_eq!(connection_count, 2);
// }