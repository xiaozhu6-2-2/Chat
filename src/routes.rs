// src/routes.rs
// 库模块导入
use axum::{
    routing::{get, post}, 
    Router, 
    middleware,
};
use tower_http::cors::{CorsLayer, Any};

// 分离模块导入
use super::handlers;
use crate::{
    middleware::{
        auth_middleware,
        ws_auth_middleware
    },
    state::AppState
};

// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    // CORS 中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any); 

    let public_routes = Router::new()
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/session-key", get(handlers::auth::get_session_key));
    
    // 需要token认证的路由
    let protected_routes = Router::new()
        // .route("/chatrooms/create", post(handlers::chatroom::create_chatroom))
        // .route("/chatrooms/join", post(handlers::chatroom::join_chatroom))
        // .route("/chatrooms/leave", post(handlers::chatroom::leave_chatroom))
        // .route("/chatrooms/joined", get(handlers::chatroom::get_joined_chatrooms))
        // .route("/online-users/{:room_id}", get(handlers::online_status::get_online_users))

        // .route("/friend-requests", post(handlers::friends::send_friend_request))
        // .route("/friend-requests", get(handlers::friends::list_friend_requests))
        // .route("/friend-requests/respond", post(handlers::friends::respond_friend_request))
        // .route("/friends", get(handlers::friends::list_friends))
        // .route("/friends/{:friend_account}", delete(handlers::friends::remove_friend))

        // .route("/private-chat/start", post(handlers::direct_conversation::start_private_chat))
        // .route("/private-chat/history/{:session_id}", get(handlers::direct_conversation::get_private_chat_history))
        .route_layer(middleware::from_fn(auth_middleware));

    let ws_route: Router<AppState> = Router::new()
        .route("/connection/ws",get(handlers::connections::websocket_handler))
        .layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
}