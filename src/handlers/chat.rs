use axum::Extension;
use axum::{extract::State, Json};

use crate::models::others::Claims;
use crate::models::requests::{GroupChatRequest, PrivateChatRequest};
use crate::models::{errors::AppResult, responses::ChatListResponse};
use crate::models::responses::{ChatItem, ChatType, GroupChatResponse, PrivateChatResponse};
use crate::state::AppState;
use crate::models::repository::{PrivateChatRepository, GroupChatRepository, UserRepository, FriendshipRepository, GroupMessageRepository};
use chrono::Utc;

pub async fn get_chat_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<ChatListResponse>> {
    // 从 claims 的 sub 中获取用户 account
    let account = &claims.sub;

    // 根据 account 获取用户信息
    let user = state.db_pool.find_user_by_account(account).await?;
    let uid = user.uid;

    let mut chat_items = Vec::new();

    // 1. 获取私聊会话
    let private_chats = state.db_pool.find_chats_by_user(&uid).await?;

    for chat in private_chats {
        // 确定对方用户 ID
        let other_uid = if chat.uid1 == uid { &chat.uid2 } else { &chat.uid1 };

        // 查找好友关系
        if let Some(friendship) = state.db_pool.find_friendship_by_users(&uid, other_uid).await? {
            // 获取未读消息数
            let unread_count = state.db_pool.get_unread_message_count_by_chat(&chat.pid, &uid).await?;

            // 获取最新消息
            let latest_msg = state.db_pool.find_latest_message_by_chat(&chat.pid).await?;

            // 获取对方用户信息
            let other_user = state.db_pool.find_user_by_uid(other_uid).await?;

            // 检查置顶状态
            let is_pinned = if uid == chat.uid1 {
                chat.is_pinned_by_uid1.unwrap_or(0) == 1
            } else {
                chat.is_pinned_by_uid2.unwrap_or(0) == 1
            };

            // 构建备注/显示名称
            let remark = if uid == friendship.uid {
                friendship.remark.as_deref().unwrap_or(&other_user.username)
            } else {
                friendship.to_remark.as_deref().unwrap_or(&other_user.username)
            };

            // 构建头像URL
            let avatar = other_user.avatar.as_deref().unwrap_or("").to_string();

            // 获取最新消息内容和时间戳
            let (latest_message, updated_at) = if let Some(msg) = latest_msg {
                let content = match msg.mes_type {
                    crate::models::entities::PrivateMsgType::Text => msg.content.clone(),
                    crate::models::entities::PrivateMsgType::Image => "[图片]".to_string(),
                    crate::models::entities::PrivateMsgType::File => "[文件]".to_string(),
                    crate::models::entities::PrivateMsgType::Voice => "[语音]".to_string(),
                    crate::models::entities::PrivateMsgType::Video => "[视频]".to_string(),
                    crate::models::entities::PrivateMsgType::Link => "[链接]".to_string(),
                    crate::models::entities::PrivateMsgType::Emoji => "[表情]".to_string(),
                    crate::models::entities::PrivateMsgType::Annoucement => "[公告]".to_string(),
                };
                let timestamp = msg.send_time
                    .map(|dt| dt.timestamp())
                    .unwrap_or_else(|| Utc::now().timestamp());
                (content, timestamp)
            } else {
                ("暂无消息".to_string(), Utc::now().timestamp())
            };

            // 检查是否需要添加到列表：有未读消息或置顶的会话
            if unread_count > 0 || is_pinned {
                chat_items.push(ChatItem {
                    id: chat.pid.clone(),
                    is_pinned,
                    chat_type: ChatType::Private,
                    latest_message,
                    updated_at,
                    unread_messages: unread_count,
                    avatar,
                    remark: remark.to_string(),
                });
            }
        }
    }

    // 2. 获取群聊会话
    let group_memberships = state.db_pool.find_groups_by_user(&uid).await?;

    for membership in group_memberships {
        // 获取群聊信息
        if let Some(group_chat) = state.db_pool.find_group_by_gid(&membership.gid).await? {
            // 获取未读消息数
            let unread_count = state.db_pool.get_unread_message_count_by_group(&membership.gid, &uid).await?;

            // 获取最新消息
            let latest_msg = state.db_pool.find_latest_message_by_group(&membership.gid).await?;

            // 检查置顶状态
            let is_pinned = membership.is_pinned.unwrap_or(0) == 1;

            // 构建备注/显示名称
            let remark = membership.remark.as_deref().unwrap_or(&group_chat.group_name);

            // 构建头像URL
            let avatar = group_chat.group_avatar.as_deref().unwrap_or("").to_string();

            // 获取最新消息内容和时间戳
            let (latest_message, updated_at) = if let Some(msg) = latest_msg {
                let content = match msg.msg_type {
                    crate::models::entities::GroupMsgType::Text => msg.content.clone(),
                    crate::models::entities::GroupMsgType::Image => "[图片]".to_string(),
                    crate::models::entities::GroupMsgType::File => "[文件]".to_string(),
                    crate::models::entities::GroupMsgType::Voice => "[语音]".to_string(),
                    crate::models::entities::GroupMsgType::Video => "[视频]".to_string(),
                    crate::models::entities::GroupMsgType::Link => "[链接]".to_string(),
                    crate::models::entities::GroupMsgType::Emoji => "[表情]".to_string(),
                    crate::models::entities::GroupMsgType::Annoucement => "[公告]".to_string(),
                };
                let timestamp = msg.send_time
                    .map(|dt| dt.timestamp())
                    .unwrap_or_else(|| Utc::now().timestamp());
                (content, timestamp)
            } else {
                ("暂无消息".to_string(), Utc::now().timestamp())
            };

            // 检查是否需要添加到列表：有未读消息或置顶的会话
            if unread_count > 0 || is_pinned {
                chat_items.push(ChatItem {
                    id: membership.gid.clone(),
                    is_pinned,
                    chat_type: ChatType::Group,
                    latest_message,
                    updated_at,
                    unread_messages: unread_count,
                    avatar,
                    remark: remark.to_string(),
                });
            }
        }
    }

    // 按置顶状态和时间戳排序（置顶的在前，然后按时间戳降序）
    chat_items.sort_by(|a, b| {
        match (a.is_pinned, b.is_pinned) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.updated_at.cmp(&a.updated_at),
        }
    });

    let total = chat_items.len() as i32;
    
    Ok(Json(ChatListResponse {
        chats: chat_items,
        total,
    }))
}

pub async fn get_private_chat(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PrivateChatRequest>,
) -> AppResult<Json<PrivateChatResponse>> {
    // 从 claims 的 sub 中获取用户 account
    let account = &claims.sub;

    // 根据 account 获取用户信息
    let user = state.db_pool.find_user_by_account(account).await?;
    let uid = user.uid;

    // 根据 fid 查找好友关系
    let friendship = state.db_pool.find_friendship_by_fid(&payload.fid).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound("Friendship not found".to_string()))?;

    // 验证当前用户是否是好友关系中的一方
    if friendship.uid != uid && friendship.to_uid != uid {
        return Err(crate::models::errors::AppError::Forbidden("Access denied: You are not part of this friendship".to_string()));
    }

    // 确定对方用户 ID
    let other_uid = if friendship.uid == uid { &friendship.to_uid } else { &friendship.uid };

    // 查找私聊会话
    let chat = state.db_pool.find_chat_by_users(&uid, other_uid).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound("Private chat not found".to_string()))?;

    // 获取对方用户信息
    let other_user = state.db_pool.find_user_by_uid(other_uid).await?;

    // 检查置顶状态
    let is_pinned = if uid == chat.uid1 {
        chat.is_pinned_by_uid1.unwrap_or(0) == 1
    } else {
        chat.is_pinned_by_uid2.unwrap_or(0) == 1
    };

    // 构建备注/显示名称
    let remark = if uid == friendship.uid {
        friendship.remark.as_deref().unwrap_or(&other_user.username)
    } else {
        friendship.to_remark.as_deref().unwrap_or(&other_user.username)
    };

    // 构建头像URL
    let avatar = other_user.avatar.as_deref().unwrap_or("").to_string();

    // 获取最新消息
    let latest_msg = state.db_pool.find_latest_message_by_chat(&chat.pid).await?;

    // 获取最新消息内容和时间戳
    let (latest_message, updated_at) = if let Some(msg) = latest_msg {
        let content = match msg.mes_type {
            crate::models::entities::PrivateMsgType::Text => msg.content.clone(),
            crate::models::entities::PrivateMsgType::Image => "[图片]".to_string(),
            crate::models::entities::PrivateMsgType::File => "[文件]".to_string(),
            crate::models::entities::PrivateMsgType::Voice => "[语音]".to_string(),
            crate::models::entities::PrivateMsgType::Video => "[视频]".to_string(),
            crate::models::entities::PrivateMsgType::Link => "[链接]".to_string(),
            crate::models::entities::PrivateMsgType::Emoji => "[表情]".to_string(),
            crate::models::entities::PrivateMsgType::Annoucement => "[公告]".to_string(),
        };
        let timestamp = msg.send_time
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());
        (content, timestamp)
    } else {
        ("暂无消息".to_string(), Utc::now().timestamp())
    };

    Ok(Json(PrivateChatResponse {
        id: chat.pid,
        is_pinned,
        chat_type: "private".to_string(),
        latest_message,
        updated_at,
        avatar,
        remark: remark.to_string(),
    }))
}

pub async fn get_group_chat(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupChatRequest>,
) -> AppResult<Json<GroupChatResponse>> {
    // 从 claims 的 sub 中获取用户 account
    let account = &claims.sub;

    // 根据 account 获取用户信息
    let user = state.db_pool.find_user_by_account(account).await?;
    let uid = user.uid;

    // 验证用户是否在群组中
    let membership = state.db_pool.find_member(&payload.gid, &uid).await?
        .ok_or_else(|| crate::models::errors::AppError::Forbidden("You are not a member of this group".to_string()))?;

    // 获取群组信息
    let group_chat = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound("Group not found".to_string()))?;

    // 检查置顶状态
    let is_pinned = membership.is_pinned.unwrap_or(0) == 1;

    // 构建备注/显示名称
    let remark = membership.remark.as_deref().unwrap_or(&group_chat.group_name);

    // 构建头像URL
    let avatar = group_chat.group_avatar.as_deref().unwrap_or("").to_string();

    // 获取最新消息
    let latest_msg = state.db_pool.find_latest_message_by_group(&payload.gid).await?;

    // 获取最新消息内容和时间戳
    let (latest_message, updated_at) = if let Some(msg) = latest_msg {
        let content = match msg.msg_type {
            crate::models::entities::GroupMsgType::Text => msg.content.clone(),
            crate::models::entities::GroupMsgType::Image => "[图片]".to_string(),
            crate::models::entities::GroupMsgType::File => "[文件]".to_string(),
            crate::models::entities::GroupMsgType::Voice => "[语音]".to_string(),
            crate::models::entities::GroupMsgType::Video => "[视频]".to_string(),
            crate::models::entities::GroupMsgType::Link => "[链接]".to_string(),
            crate::models::entities::GroupMsgType::Emoji => "[表情]".to_string(),
            crate::models::entities::GroupMsgType::Annoucement => "[公告]".to_string(),
        };
        let timestamp = msg.send_time
            .map(|dt| dt.timestamp())
            .unwrap_or_else(|| Utc::now().timestamp());
        (content, timestamp)
    } else {
        ("暂无消息".to_string(), Utc::now().timestamp())
    };

    Ok(Json(GroupChatResponse {
        id: payload.gid,
        is_pinned,
        chat_type: "group".to_string(),
        latest_message,
        updated_at,
        avatar,
        remark: remark.to_string(),
    }))
}