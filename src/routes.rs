// src/routes.rs
// 库模块导入
use axum::{
    routing::{get, post}, 
    Router, 
    middleware,
    extract::{Path, State},
    Extension
};
use tower_http::cors::{CorsLayer, Any};
use axum::http::{Method, HeaderName};
use axum::extract::ws::WebSocketUpgrade;

// 分离模块导入
use super::handlers;
use crate::{
    middleware::{auth_middleware, ws_auth_middleware},
    state::AppState,
    handlers::handle_websocket,
    models::Claims
};


// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    // CORS 中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(vec![Method::GET, Method::POST])
        .allow_headers(vec![HeaderName::from_static("content-type")]); 

    let public_routes = Router::new()
        .route("/", get(handlers::root))
        .route("/register", post(handlers::register))
        .route("/login", post(handlers::login));
    
    let protected_routes = Router::new() // 被保护的路由
        .route("/protected", get(handlers::protected))
        .route("/chatrooms/create", post(handlers::create_chatroom))
        .route("/chatrooms/join", post(handlers::join_chatroom))
        .route("/chatrooms/leave", post(handlers::leave_chatroom))
        .route_layer(middleware::from_fn(auth_middleware));

    let ws_route = Router::new().route(
        "/ws/{:room_id}",
        get(|ws: WebSocketUpgrade, Path(room_id): Path<u32>, State(state): State<AppState>, Extension(claims): Extension<Claims>| async move {
            ws.on_upgrade(move |socket| handle_websocket(
                Path(room_id),
                socket, 
                State(state),
                Extension(claims)
            ))
        })
        .route_layer(middleware::from_fn(ws_auth_middleware))
    );

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
}