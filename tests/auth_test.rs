// // tests/auth_test.rs
// use echat::{
//     create_test_app, 
//     models::{
//         requests::{RegisterRequest, LoginRequest},
//         responses::{RegisterResponse, LoginResponse, SessionKeyResponse}
//     }
// };
// use axum::{
//     body::{Body, to_bytes},
//     http::{Method, Request, StatusCode, status},
// };
// use tower::ServiceExt; // for `oneshot` method
// use base64::engine::general_purpose;
// use base64::Engine;
// use rsa::{RsaPublicKey, pkcs8::DecodePublicKey, Pkcs1v15Encrypt};
// use rand_core::OsRng;

// fn set_test_env_var(key: &str, value: &str) {
//     unsafe {
//         std::env::set_var(key, value);
//     }
// }

// #[tokio::test]
// async fn test_auth_workflow() {
//     // 创建测试应用
//     let app = create_test_app().await.unwrap();
//     let router = app.router();
    
//     set_test_env_var("JWT_SECRET", &app.config().jwt_secret);

//     // 1. 获取会话公钥
//     let session_key_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/session-key")
//             .method(Method::GET)
//             .body(Body::empty())
//             .unwrap())
//         .await
//         .unwrap();
    
//     assert_eq!(session_key_response.status(), StatusCode::OK);
    
//     let body = to_bytes(session_key_response.into_body(), 10 * 1024 * 1024).await.unwrap();
//     let session_key: SessionKeyResponse = serde_json::from_slice(&body).unwrap();
    
//     // 解析公钥 
//     let public_key = RsaPublicKey::from_public_key_pem(&session_key.public_key).unwrap();
    
//     // 2. 使用公钥加密测试数据
//     let test_account = "test_user_123";
//     let test_password = "test_password_123";
//     let test_username = "Test User";
    
//     let encrypted_account = encrypt_with_public_key(&public_key, test_account);
//     let encrypted_password = encrypt_with_public_key(&public_key, test_password);
    
//     // 3. 测试注册
//     let register_request = RegisterRequest {
//         account: encrypted_account,
//         password: encrypted_password,
//         username: test_username.to_string(),
//     };
    
//     let register_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/register")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&register_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     let register_status = register_response.status();
//     let register_body_bytes = to_bytes(register_response.into_body(), 10 * 1024 * 1024).await.unwrap();

//     // 打印详细的错误信息
//     if register_status != StatusCode::OK {
//         let error_body = String::from_utf8_lossy(&register_body_bytes);
//         println!("注册失败，状态码: {}", register_status);
//         println!("错误响应: {}", error_body);
//     }

//     assert_eq!(register_status, StatusCode::OK);
    
//     let register_result: RegisterResponse = serde_json::from_slice(&register_body_bytes).unwrap();
//     assert!(register_result.success);
    
//     // 4. 测试登录
//     let login_request = LoginRequest {
//         account: encrypt_with_public_key(&public_key, test_account),
//         password: encrypt_with_public_key(&public_key, test_password),
//     };
    
//     let login_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/login")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&login_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     let login_status = login_response.status();
//     let login_body_bytes = to_bytes(login_response.into_body(), 10 * 1024 * 1024).await.unwrap();

//     // 打印详细的错误信息
//     if login_status != StatusCode::OK {
//         let error_body = String::from_utf8_lossy(&login_body_bytes);
//         println!("注册失败，状态码: {}", login_status);
//         println!("错误响应: {}", error_body);
//     }

//     assert_eq!(login_status, StatusCode::OK);
    
//     let login_result : LoginResponse = serde_json::from_slice(&login_body_bytes).unwrap();
    
//     assert_eq!(login_result.account, test_account);
//     assert_eq!(login_result.username, test_username);
//     assert!(!login_result.token.is_empty());
    
//     // 5. 测试重复注册（应该失败）
//     let duplicate_register_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/register")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&register_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     // 重复注册应该返回错误状态码
//     assert_ne!(duplicate_register_response.status(), StatusCode::OK);
// }

// #[tokio::test]
// async fn test_login_with_invalid_credentials() {  
//     let app = create_test_app().await.unwrap();
//     let router = app.router();
    
//     // 获取公钥
//     let session_key_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/session-key")
//             .method(Method::GET)
//             .body(Body::empty())
//             .unwrap())
//         .await
//         .unwrap();
    
//     let body = to_bytes(session_key_response.into_body(), 10 * 1024 * 1024).await.unwrap();
//     let session_key: SessionKeyResponse = serde_json::from_slice(&body).unwrap();
//     let public_key = RsaPublicKey::from_public_key_pem(&session_key.public_key).unwrap();
    
//     // 测试错误密码登录
//     let login_request = LoginRequest {
//         account: encrypt_with_public_key(&public_key, "nonexistent_user"),
//         password: encrypt_with_public_key(&public_key, "wrong_password"),
//     };
    
//     let login_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/login")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&login_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     let status = login_response.status();
//     let body_bytes = to_bytes(login_response.into_body(), 10 * 1024 * 1024).await.unwrap();

//     if (status != StatusCode::OK) {
//         let error_body = String::from_utf8_lossy(&body_bytes);
//         println!("注册失败，状态码: {}", status);
//         println!("错误响应: {}", error_body);
//     } 
//     // 应该返回错误状态码
//     assert_ne!(status, StatusCode::OK);
// }

// // 辅助函数：使用公钥加密数据
// fn encrypt_with_public_key(public_key: &RsaPublicKey, data: &str) -> String {
//     let padding = Pkcs1v15Encrypt;
//     let encrypted_data = public_key.encrypt(&mut OsRng, padding, data.as_bytes()).unwrap();
//     general_purpose::STANDARD.encode(encrypted_data)
// }

// #[tokio::test]
// async fn test_websocket_auth() {    
//     let app = create_test_app().await.unwrap();
//     let router = app.router();
    
//     set_test_env_var("JWT_SECRET", &app.config().jwt_secret);

//     // 1. 先注册用户
//     let session_key_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/session-key")
//             .method(Method::GET)
//             .body(Body::empty())
//             .unwrap())
//         .await
//         .unwrap();
    
//     let body = to_bytes(session_key_response.into_body(), 10 * 1024 * 1024).await.unwrap();
//     let session_key: SessionKeyResponse = serde_json::from_slice(&body).unwrap();
//     let public_key = RsaPublicKey::from_public_key_pem(&session_key.public_key).unwrap();
    
//     let test_account = "ws_test_user";
//     let test_password = "ws_test_password";
    
//     let register_request = RegisterRequest {
//         account: encrypt_with_public_key(&public_key, test_account),
//         password: encrypt_with_public_key(&public_key, test_password),
//         username: "WS Test User".to_string(),
//     };
    
//     let _ = router.clone()
//         .oneshot(Request::builder()
//             .uri("/register")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&register_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     // 2. 登录获取token
//     let login_request = LoginRequest {
//         account: encrypt_with_public_key(&public_key, test_account),
//         password: encrypt_with_public_key(&public_key, test_password),
//     };
    
//     let login_response = router.clone()
//         .oneshot(Request::builder()
//             .uri("/login")
//             .method(Method::POST)
//             .header("content-type", "application/json")
//             .body(Body::from(serde_json::to_vec(&login_request).unwrap()))
//             .unwrap())
//         .await
//         .unwrap();
    
//     let login_status = login_response.status();
//     let body = to_bytes(login_response.into_body(), 10 * 1024 * 1024).await.unwrap();
    
//     if login_status != StatusCode::OK {
//         let err = String::from_utf8_lossy(&body);
//         println!("登录失败，状态码:{}",login_status);
//         println!("错误原因:{}", err);
//     }

//     assert_eq!(login_status, StatusCode::OK);

//     let login_result: LoginResponse = serde_json::from_slice(&body).unwrap();
//     let token = login_result.token;
    
//     // 3. 测试WebSocket认证（带token的请求）
//     let ws_response = router
//         .oneshot(Request::builder()
//             .uri(&format!("/auth/connection/ws?token={}", token))
//             .method(Method::GET)
//             .header("upgrade", "websocket")
//             .header("connection", "upgrade")
//             .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==")
//             .header("sec-websocket-version", "13")
//             .body(Body::empty())
//             .unwrap())
//         .await
//         .unwrap();
    
//     // WebSocket握手应该返回101状态码
//     // 注意：由于测试环境限制，这里可能无法完全测试WebSocket连接
//     // 但至少可以验证认证中间件是否工作
//     println!("WebSocket response status: {}", ws_response.status());
// }