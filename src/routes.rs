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
    handlers::user::get_user_info, middleware::{
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
    let protected_routes = Router::new();
        // .route("/auth/user/user-info", post(handlers::user::get_user_info))// 获取用户信息
        // .route("/auth/user/update-user-info", post(handler))// 更新用户信息
        // .route("/auth/user/profile", post())// 获取非好友用户资料

        // .route("/auth/chat/list", get(handler))// 获取会话列表
        // .route("/auth/chat/soloprivate", post(handler))// 获取指定私聊会话
        // .route("/auth/chat/sologroup", post())// 获取指定群聊会话

        // .route("/auth/message/private_history", post(handler))// 获取私聊会话历史消息
        // .route("/auth/message/group_history", post(handler))// 获取群聊会话历史消息
        // .route("/auth/message/read", post(handler))// 设置消息为已读

        // .route("/auth/friends/search", post())// 搜索用户
        // .route("/auth/friends/profile", post())// 获取好友资料
        // .route("/auth/friends/friendlist", post())// 获取好友列表
        // .route("/auth/friends/request", post())// 发送好友请求
        // .route("/auth/friends/respond", post())// 回复好友请求
        // .route("/auth/friends/request_list", post())// 获取好友请求列表
        // .route("/auth/friends/remove", post())// 删除好友
        // .route("/auth/friends/update-remark", post())// 更新好友备注
        // .route("/auth/friends/blacklist", post())// 更新好友黑名单状态

        // .route("/auth/groups/search", post())// 搜索群聊
        // .route("/auth/groups/profile", post())// 获取群聊资料
        // .route("/auth/groups/grouplist", post())// 获取群聊列表

        // .route("/auth/state/friends-online", post())// 获取好友列表在线状态
        // .route("/auth/state/group-online", post())// 获取群聊在线状态

        // .route("/auth/file/upload", post())// 上传文件
        // .route("/auth/file/preview", post())// 预览文件
        // .route("/auth/file/download", post())// 下载文件
        // .route("/auth/file/delete", post())// 删除文件

        // .route_layer(middleware::from_fn(auth_middleware));
        
        // .route("/auth/group/profile", post())// 获取群聊资料
        // .route("/auth/group/profile", post())// 获取群聊资料
        // .route("/auth/group/profile", post())// 获取群聊资料
        

    let ws_route: Router<AppState> = Router::new()
        .route("/auth/connection/ws",post(handlers::connections::websocket_handler))
        .layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
}