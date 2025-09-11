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
use axum::extract::ws::WebSocketUpgrade;
use axum::routing::delete;

// 分离模块导入
use super::handlers;
use crate::{
    middleware::{auth_middleware, ws_auth_middleware},
    state::AppState,
    models::others::Claims
};

// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    // CORS 中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any); 

    let public_routes = Router::new()
        .route("/", get(handlers::auth::login))
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/auth/session-key", get(handlers::auth::get_session_key));
    
    let protected_routes = Router::new() // 被保护的路由
        .route("/chatrooms/create", post(handlers::chatroom::create_chatroom))
        .route("/chatrooms/join", post(handlers::chatroom::join_chatroom))
        .route("/chatrooms/leave", post(handlers::chatroom::leave_chatroom))
        .route("/chatrooms/joined", get(handlers::chatroom::get_joined_chatrooms))
        .route("/online-users/{:room_id}", get(handlers::online_status::get_online_users))

        .route("/friend-requests", post(handlers::friends::send_friend_request))
        .route("/friend-requests", get(handlers::friends::list_friend_requests))
        .route("/friend-requests/respond", post(handlers::friends::respond_friend_request))
        .route("/friends", get(handlers::friends::list_friends))
        .route("/friends/{:friend_account}", delete(handlers::friends::remove_friend))

        .route("/private-chat/start", post(handlers::direct_conversation::start_private_chat))
        .route("/private-chat/history/{:session_id}", get(handlers::direct_conversation::get_private_chat_history))
        .route_layer(middleware::from_fn(auth_middleware));

    let ws_route = Router::new().route(
        "/ws/{:room_id}",
        get(|ws: WebSocketUpgrade, Path(room_id): Path<u32>, State(state): State<AppState>, Extension(claims): Extension<Claims>| async move {
            ws.on_upgrade(move |socket| handlers::trans_logic::handle_websocket(
                Path(room_id),
                socket, 
                State(state),
                Extension(claims)
            ))
        })
        .route_layer(middleware::from_fn(ws_auth_middleware))
    );

    let private_ws_route = Router::new().route(
        "/private-chat/ws/{:session_id}",
        get(handlers::trans_logic::handle_private_websocket)
    ).route_layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .merge(private_ws_route)
        .layer(cors)
}