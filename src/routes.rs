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
        // // 群聊管理API
        // .route("/auth/group_chat/create", post(handlers::chat_group::create_group))         // 创建群聊
        // .route("/auth/group_chat/exit", post(handlers::chat_group::exit_group))             // 退出群聊
        // .route("/auth/group_chat/dismiss", post(handlers::chat_group::dismiss_group))       // 解散群聊
        // .route("/auth/group_chat/translate", post(handlers::chat_group::translate_group))   // 转让群聊
        // .route("/auth/group_chat/set_role", post(handlers::chat_group::set_role))           // 设置权限
        // .route("/auth/group_chat/invite", post(handlers::chat_group::invite))               // 邀请成员
        // .route("/auth/group_chat/kick", post(handlers::chat_group::kick))                   // 踢出群聊
        // .route("/auth/group_chat/online_state", post(handlers::chat_group::online_state))   // 群成员在线状态
        // .route("/auth/group_chat/notification", post(handlers::chat_group::notification))   // 群公告
        // .route("/auth/group_chat/group_rename", post(handlers::chat_group::group_rename))   // 修改群名称
        // .route("/auth/group_chat/remark", post(handlers::chat_group::remark))               // 修改群备注
        // .route("/auth/group_chat/reavator", post(handlers::chat_group::reavator))           // 修改群头像
        // .route("/auth/group_chat/info", post(handlers::chat_group::info))                   // 获取群信息
        // .route("/auth/group_chat/history", post(handlers::chat_group::history))             // 获取历史消息
        // .route("/auth/group_chat/member_rename", post(handlers::chat_group::member_rename)) // 修改群昵称
        // .route("/auth/group_chat/tag", post(handlers::chat_group::tag))                     // 修改群标签
        // // 私聊管理API

        // // 好友管理API
        // .route("/auth/friends/add", post(handlers::friends::add))                           // 添加好友
        // .route("/auth/friends/delete", post(handlers::friends::delete))                     // 删除好友
        // .route("/auth/friends/black_list", post(handlers::friends::))                       // 拉入黑名单
        // .route("/auth/friends/remark", post(handlers::friends::remark))                     // 修改好友备注
        // .route("/auth/friends/tag", post(handlers::friends::tag))                           // 修改好友分组
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