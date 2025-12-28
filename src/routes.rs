// src/routes.rs
// 库模块导入
use axum::{
    routing::{get, post},
    Router,
    middleware,
    extract::DefaultBodyLimit,
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
        .route("/auth/user/validate", get(handlers::user::validate))// 验证用户token是否有效
        .route("/auth/user/user-info", get(handlers::user::get_user_info))// 获取用户信息
        .route("/auth/user/update-user-info", post(handlers::user::update_user_info))// 更新用户信息
        .route("/auth/user/update-user-avatar", post(handlers::user::update_user_avatar))// 更新用户头像
        .route("/auth/user/profile", post(handlers::user::fetch_user_profile))// 获取非好友用户资料

        .route("/auth/chat/list", get(handlers::chat::get_chat_list))// 获取会话列表
        .route("/auth/chat/soloprivate", post(handlers::chat::get_private_chat))// 获取指定私聊会话
        .route("/auth/chat/sologroup", post(handlers::chat::get_group_chat))// 获取指定群聊会话
        .route("/auth/chat/updateIsPinned", post(handlers::chat::update_ispinned))// 会话置顶状态

        .route("/auth/message/private_history", post(handlers::message::get_private_history))// 获取私聊会话历史消息
        .route("/auth/message/group_history", post(handlers::message::get_group_history))// 获取群聊会话历史消息
        .route("/auth/message/read", post(handlers::message::mark_msg_read))// 设置消息为已读
        .route("/auth/message/read_count", post(handlers::message::fetch_group_read))// 获取群聊已读状态
        .route("/auth/message/revoke", post(handlers::message::revoke_message))// 撤回消息

        .route("/auth/friends/search", post(handlers::friends::search_user))// 搜索用户
        .route("/auth/friends/profile", post(handlers::friends::get_friend_profile))// 获取好友资料
        .route("/auth/friends/friendlist", get(handlers::friends::get_friend_list))// 获取好友列表
        .route("/auth/friends/request", post(handlers::friends::send_friend_request))// 发送好友请求
        .route("/auth/friends/respond", post(handlers::friends::respond_friend_request))// 回复好友请求
        .route("/auth/friends/request_list", get(handlers::friends::get_friend_request_list))// 获取好友请求列表
        .route("/auth/friends/remove", post(handlers::friends::remove_friend))// 删除好友
        .route("/auth/friends/update", post(handlers::friends::update_friend_remark_blacklist_group))// 更新好友备注/黑名单/分组

        .route("/auth/groups/create", post(handlers::groups::create_group))// 创建群聊
        .route("/auth/groups/search", post(handlers::groups::search_group))// 搜索群聊
        .route("/auth/groups/card", post(handlers::groups::get_group_card))//获取群聊名片
        .route("/auth/groups/profile", post(handlers::groups::get_group_profile))// 获取群聊资料
        .route("/auth/groups/grouplist", get(handlers::groups::get_group_list))// 获取群聊列表
        .route("/auth/groups/send_group_request", post(handlers::groups::send_group_request))// 发送加入群聊申请
        .route("/auth/groups/get_request_list", get(handlers::groups::get_request_list))//查看群聊申请列表（用户）
        .route("/auth/groups/group_requests", post(handlers::groups::group_requests))// 获取群聊的申请列表
        .route("/auth/groups/group_request_list", get(handlers::groups::get_group_requestlist))// 获取群聊的申请列表
        .route("/auth/groups/respond", post(handlers::groups::handle_group_request))// 处理加入群聊申请
        .route("/auth/groups/leave", post(handlers::groups::leave_group))// 退出群聊
        .route("/auth/groups/kick_member", post(handlers::groups::kick_member))// 踢出群成员
        .route("/auth/groups/disband", post(handlers::groups::disband_group))// 解散群聊
        .route("/auth/groups/member_set", post(handlers::groups::member_set))// 群聊成员修改本地设置
        .route("/auth/groups/setting", post(handlers::groups::set_group))// 修改群聊资料
        .route("/auth/groups/setting_avatar", post(handlers::groups::set_group_avatar))// 修改群聊头像
        .route("/auth/groups/get_announcements", post(handlers::groups::get_announcements))//获取群公告列表
        .route("/auth/groups/get_members", post(handlers::groups::get_members))//获取群成员列表
        .route("/auth/groups/transfer_ownership", post(handlers::groups::transfer_ownership))//转让群主
        .route("/auth/groups/set_admin", post(handlers::groups::set_admin))//设置管理员
        .route("/auth/groups/remove_admin", post(handlers::groups::remove_admin))//设置管理员
        .route("/auth/groups/get_ban_status", post(handlers::groups::get_ban_status))//获取用户禁言状态
        .route("/auth/groups/ban_member", post(handlers::groups::ban_member))//禁言群成员
        .route("/auth/groups/remove_mute_admin", post(handlers::groups::remove_mute_admin))//解除禁言
        .route("/auth/online/friends-online", get(handlers::online::get_friends_online))// 获取好友列表在线状态
        .route("/auth/online/group-online", post(handlers::online::get_group_online))// 获取群聊在线状态

        .route("/auth/file/upload", post(handlers::file::upload_file))// 上传文件
        .route("/auth/file/preview", post(handlers::file::preview_file))// 预览文件
        .route("/auth/file/download", post(handlers::file::download_file))// 下载文件
        .route("/auth/file/delete", post(handlers::file::delete_file))// 删除文件

        .route_layer(middleware::from_fn(auth_middleware));

    let ws_route: Router<AppState> = Router::new()
        .route("/auth/connection/ws",get(handlers::connections::websocket_handler))
        .layer(middleware::from_fn(ws_auth_middleware));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_route)
        .layer(cors)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 1024)) // 100MB
}
