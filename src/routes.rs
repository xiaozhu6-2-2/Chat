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
        .route("/register", post(handlers::auth::register))
        .route("/login", post(handlers::auth::login))
        .route("/session-key", get(handlers::auth::get_session_key));
    
    // 需要token认证的路由
    let protected_routes = Router::new();
        // // 群聊管理类API
        // .route("/auth/group_chat/info", get(handlers::chat_group::))// 获取群聊信息
        // .route("/auth/group_chat/update", put(handlers::chat_group::)) // 更新群聊信息
        // .route("/auth/group_chat/dismiss", delete(handlers::chat_group::))// 解散群聊
        // .route("/auth/group_chat/transfer", post(handlers::chat_group::))// 转让群主
        // // 群聊成员管理类API
        // .route("/auth/group_chat/kick", post(handlers::chat_group::kick_member))// 踢出群成员
        // .route("/auth/group_chat/members", get(handlers::chat_group::get_members))// 获取群成员列表
        // .route("/auth/group_chat/member/update", put(handlers::chat_group::update_member_info))// 修改群成员昵称/备注
        // .route("/auth/group_chat/invite", post(handlers::chat_group::invite_member))// // 邀请用户加入群聊
        // // 申请处理类API
        // .route("/auth/group_chat/requests", get(handlers::chat_group::get_join_requests))// 获取加群申请列表
        // .route("/auth/group_chat/request/handle", post(handlers::chat_group::handle_join_request))// 处理加群申请（同意/拒绝）

        // .route("/auth/group_chat/join", post(handlers::chat_group::))// 加入群聊
        // .route("/auth/group_chat/leave", post(handlers::chat_group::))
        // .route("/auth/group_chat/mute", post(handlers::chat_group::))
        // .route("/auth/group_chat/set_announce", post(handlers::chat_group::))
        // .route("/auth/group_chat/set_role", post(handlers::chat_group::))
        // .route("/auth/group_chat/pull_history", post(handlers::chat_group::))
        // .route("/friend-requests", post(handlers::friends::send_friend_request))
        // .route("/friend-requests", get(handlers::friends::list_friend_requests))
        // .route("/friend-requests/respond", post(handlers::friends::respond_friend_request))
        // .route("/friends", get(handlers::friends::list_friends))
        // .route("/friends/{:friend_account}", delete(handlers::friends::remove_friend))

        // .route("/private-chat/start", post(handlers::direct_conversation::start_private_chat))
        // .route("/private-chat/history/{:session_id}", get(handlers::direct_conversation::get_private_chat_history))
        // .route_layer(middleware::from_fn(auth_middleware));

    let ws_route: Router<AppState> = Router::new()
        .route("/auth/connection/ws",get(handlers::connections::websocket_handler))
        .layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
}