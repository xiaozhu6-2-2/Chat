// src/routes.rs
// 库模块导入
use axum::{
    routing::{get, post},
    Router,
};

// 分离模块导入
use super::handlers;
use crate::state::AppState;

// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    Router::new()
        .route("/", get(handlers::root))
        .route("/users", post(handlers::create_user))
}