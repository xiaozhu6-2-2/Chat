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
    }, state::AppState
};

// 构建路由并返回 Router 实例
pub fn create_routes() -> Router<AppState> {
    // CORS 中间件
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any); 

    let public_routes = Router::new()
        .route("/noauth/auth/register", post(handlers::auth::register))
        .route("/noauth/auth/login", post(handlers::auth::login))
        .route("/noauth/auth/session-key", get(handlers::auth::get_session_key));
    
    // 需要token认证的路由
    let protected_routes = Router::new()
        .route("/auth/user/user-info", get(handlers::user::get_user_info))// 获取用户信息
        .route("/auth/user/update-user-info", post(handlers::user::update_user_info))// 更新用户信息
        .route("/auth/user/profile", post(handlers::user::fetch_user_profile))// 获取非好友用户资料

        // .route("/auth/chat/list", get(handlers::chat::get_chat_list))// 获取会话列表
        // .route("/auth/chat/soloprivate", post(handlers::chat::get_private_chat))// 获取指定私聊会话
        // .route("/auth/chat/sologroup", post(handlers::chat::get_group_chat))// 获取指定群聊会话

        // .route("/auth/message/private_history", post(handlers::message::get_private_history))// 获取私聊会话历史消息
        // .route("/auth/message/group_history", post(handlers::message::get_group_history))// 获取群聊会话历史消息
        // .route("/auth/message/read", post(handlers::message::mark_msg_read))// 设置消息为已读

        .route("/auth/friends/search", post(handlers::friends::search_user))// 搜索用户
        .route("/auth/friends/profile", post(handlers::friends::get_friend_profile))// 获取好友资料
        .route("/auth/friends/friendlist", get(handlers::friends::get_friend_list))// 获取好友列表
        .route("/auth/friends/request", post(handlers::friends::send_friend_request))// 发送好友请求
        // .route("/auth/friends/respond", post(handlers::friends::respond_friend_request))// 回复好友请求
        // .route("/auth/friends/request_list", post(handlers::friends::get_friend_request_list))// 获取好友请求列表
        // .route("/auth/friends/remove", post(handlers::friends::remove_friend))// 删除好友
        // .route("/auth/friends/update-remark", post(handlers::friends::update_friend_remark))// 更新好友备注
        // .route("/auth/friends/blacklist", post(handlers::friends::update_friend_blacklist))// 更新好友黑名单状态

        // .route("/auth/groups/search", post(handlers::groups::search_group))// 搜索群聊
        .route("/auth/groups/profile", post(handlers::groups::get_group_profile))// 获取群聊资料
        // .route("/auth/groups/grouplist", post(handlers::groups::get_group_list))// 获取群聊列表

        // .route("/auth/online/friends-online", post(handlers::online::get_friends_online))// 获取好友列表在线状态
        // .route("/auth/online/group-online", post(handlers::online::get_group_online))// 获取群聊在线状态

        // .route("/auth/file/upload", post(handlers::file::upload_file))// 上传文件
        // .route("/auth/file/preview", post(handlers::file::preview_file))// 预览文件
        // .route("/auth/file/download", post(handlers::file::download_file))// 下载文件
        // .route("/auth/file/delete", post(handlers::file::delete_file))// 删除文件

        .route_layer(middleware::from_fn(auth_middleware));

    let ws_route: Router<AppState> = Router::new()
        .route("/auth/connection/ws",post(handlers::connections::websocket_handler))
        .layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
}