// src/routes.rs
// 库模块导入
use axum::{routing::{get, post}, Router};
use tower_http::cors::{CorsLayer, Any};
use axum::http::{Method, HeaderName};

// 分离模块导入
use super::handlers;
use crate::state::AppState;

// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    // CORS 中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![HeaderName::from_static("content-type")]); 

    Router::new()
        .route("/", get(handlers::root))
        .route("/login", post(handlers::login))
        .layer(cors)
}