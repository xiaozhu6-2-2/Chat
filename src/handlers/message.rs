use axum::Extension;
use axum::{extract::State, Json};

use crate::models::others::Claims;
use crate::models::errors::{AppResult, AppError};
use crate::models::responses::{FetchGroupReadResponse, GroupReadCountItem, GroupHistoryResponse, GroupMessageItem, GroupMessagePayload, PrivateHistoryResponse, PrivateMessageItem, PrivateMessagePayload, ReadResponse};
use crate::models::requests::{FetchGroupReadRequest, GroupHistoryRequest, PrivateHistoryRequest, ReadRequest};
use crate::models::entities::{PrivateMsgType, GroupMsgType};
use crate::models::repository::{FriendshipRepository, PrivateChatRepository, UserRepository, GroupChatRepository, GroupMessageRepository};
use crate::state::AppState;

pub async fn get_private_history(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PrivateHistoryRequest>,
) -> AppResult<Json<PrivateHistoryResponse>> {
    // 首先验证请求参数
    if payload.limit <= 0 {
        return Err(AppError::BadRequest("limit 必须大于 0".to_string()));
    }
    if payload.offset < 0 {
        return Err(AppError::BadRequest("offset 不能为负数".to_string()));
    }

    // 通过 account 获取当前用户信息
    let current_user = state.db_pool.find_user_by_account(&claims.sub).await?;

    // 验证用户是否属于这个私聊
    let private_chat = state.db_pool.find_chat_by_pid(&payload.pid).await?;

    let private_chat = match private_chat {
        Some(chat) => chat,
        None => {
            return Err(AppError::NotFound(format!("私聊会话 {} 不存在", payload.pid)));
        }
    };

    // 检查用户是否属于这个私聊
    if current_user.uid != private_chat.uid1 && current_user.uid != private_chat.uid2 {
        return Err(AppError::Forbidden(format!("用户 {} 不是私聊会话 {} 的参与者", current_user.uid, payload.pid)));
    }

    // 获取消息总数
    let total_messages = state.db_pool.get_message_count_by_chat(&payload.pid).await?;

    // 计算总页数（向上取整）
    let total_pages = if total_messages == 0 {
        0
    } else {
        (total_messages / payload.limit) + 1
    };

    // 检查页码是否超出范围
    if total_pages > 0 && payload.offset >= total_pages {
        return Err(AppError::PageOutOfRange {
            page: payload.offset,
            total_pages,
        });
    }

    // 获取私聊历史消息（用于分页）
    let all_messages = state.db_pool.find_messages_by_chat(&payload.pid).await?;

    let start_index = (payload.offset * payload.limit) as usize;
    let end_index = std::cmp::min(start_index + payload.limit as usize, total_messages as usize);

    // 获取当前页的消息
    let paginated_messages = if start_index < all_messages.len() {
        &all_messages[start_index..end_index]
    } else {
        &[]
    };

    // 确定对方的用户ID
    let other_uid = if current_user.uid == private_chat.uid1 {
        &private_chat.uid2
    } else {
        &private_chat.uid1
    };

    // 查询对方用户信息
    let other_user_info = if paginated_messages.iter().any(|msg| msg.sender_uid == *other_uid) {
        // 查询好友关系获取备注
        let friendship = state.db_pool.find_friendship_by_users(&current_user.uid, other_uid).await.ok().flatten();

        // 查询对方用户信息获取用户名和头像
        let other_user = state.db_pool.find_user_by_uid(other_uid).await.ok();

        (friendship, other_user)
    } else {
        (None, None)
    };

    // 转换消息格式
    let message_items: Vec<PrivateMessageItem> = paginated_messages
        .iter()
        .map(|msg| {
            // 转换消息类型
            let content_type = match msg.mes_type {
                PrivateMsgType::Text => "text",
                PrivateMsgType::Image => "image",
                PrivateMsgType::File => "file",
                PrivateMsgType::Voice => "voice",
                PrivateMsgType::Video => "video",
                PrivateMsgType::Link => "link",
                PrivateMsgType::Emoji => "emoji",
                PrivateMsgType::Annoucement => "annoucement",
            };

            // 确定 receiver_id
            let receiver_id = if msg.sender_uid == private_chat.uid1 {
                private_chat.uid2.clone()
            } else {
                private_chat.uid1.clone()
            };

            // 处理时间戳，转为 Unix 时间戳
            let timestamp = msg.send_time
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            // 处理布尔值
            let is_revoked = msg.is_revoked.unwrap_or(0) == 1;
            let is_read = msg.is_read.unwrap_or(0) == 1;

            // 获取发送者信息
            let (sender_name, sender_avatar) = if msg.sender_uid == current_user.uid {
                // 发送者是当前用户
                (
                    current_user.username.clone(),
                    current_user.avatar.clone().unwrap_or_default()
                )
            } else {
                // 发送者是对方用户
                let (ref friendship, ref other_user) = other_user_info;

                // 确定显示名称：优先使用备注，没有备注使用用户名
                let display_name = if let Some(f) = friendship {
                    match f.remark {
                        Some(ref remark) if !remark.is_empty() => remark.clone(),
                        _ => other_user.as_ref().unwrap().username.clone(),
                    }
                } else {
                    other_user.as_ref().unwrap().username.clone()
                };

                // 获取头像
                let avatar = other_user.as_ref().unwrap().avatar.clone().unwrap_or_default();

                (display_name, avatar)
            };

            // TODO: 需要在private_message表中添加quote_msg_id字段来支持引用消息功能
            let quote_msg_id: Option<String> = None;

            PrivateMessageItem {
                message_type: "Private".to_string(),
                payload: PrivateMessagePayload {
                    message_id: msg.msg_id.clone(),
                    chat_id: msg.pid.clone(),
                    timestamp,
                    sender_id: msg.sender_uid.clone(),
                    sender_name,
                    sender_avatar,
                    receiver_id,
                    content_type: content_type.to_string(),
                    detail: msg.content.clone(),
                    quote_msg_id,
                },
                is_revoked,
                is_read,
            }
        })
        .collect();

    Ok(Json(PrivateHistoryResponse {
        total_pages,
        current_page: payload.offset,
        total_items: total_messages,
        messages: message_items,
    }))
}

pub async fn get_group_history(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupHistoryRequest>,
) -> AppResult<Json<GroupHistoryResponse>> {
    // 首先验证请求参数
    if payload.limit <= 0 {
        return Err(AppError::BadRequest("limit 必须大于 0".to_string()));
    }
    if payload.offset < 0 {
        return Err(AppError::BadRequest("offset 不能为负数".to_string()));
    }

    // 通过 account 获取当前用户信息
    let current_user = state.db_pool.find_user_by_account(&claims.sub).await?;

    // 验证群聊是否存在
    let group_chat = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group_chat = match group_chat {
        Some(chat) => chat,
        None => {
            return Err(AppError::NotFound(format!("群聊 {} 不存在", payload.gid)));
        }
    };

    // 检查用户是否在群聊中
    let membership = state.db_pool.find_member(&payload.gid, &current_user.uid).await?;
    if membership.is_none() {
        return Err(AppError::Forbidden(format!("用户 {} 不是群聊 {} 的成员", current_user.uid, payload.gid)));
    }

    let member = membership.unwrap();

    // 获取用户入群时间，如果没有则使用群组创建时间
    let join_time = member.join_time
        .or(group_chat.create_time)
        .ok_or_else(|| AppError::NotFound("入群时间缺失".to_string()))?;

    // 获取所有群聊消息
    let all_messages = state.db_pool.find_messages_by_group(&payload.gid).await?;

    // 过滤出入群时间之后的消息
    let filtered_messages: Vec<_> = all_messages
        .into_iter()
        .filter(|msg| {
            msg.send_time.map_or(false, |t| t >= join_time)
        })
        .collect();

    // 获取过滤后的消息总数
    let total_messages = filtered_messages.len() as i64;

    // 计算总页数（向上取整）
    let total_pages = if total_messages == 0 {
        0
    } else {
        (total_messages / payload.limit) + 1
    };

    // 检查页码是否超出范围
    if total_pages > 0 && payload.offset >= total_pages {
        return Err(AppError::PageOutOfRange {
            page: payload.offset,
            total_pages,
        });
    }

    // 手动分页
    let start_index = (payload.offset * payload.limit) as usize;
    let end_index = std::cmp::min(start_index + payload.limit as usize, filtered_messages.len());

    let paginated_messages = if start_index < filtered_messages.len() {
        filtered_messages[start_index..end_index].to_vec()
    } else {
        Vec::new()
    };

    // 收集所有发送者UID
    let sender_uids: std::collections::HashSet<String> = paginated_messages
        .iter()
        .map(|msg| msg.sender_uid.clone())
        .collect();

    // 批量查询发送者信息
    let mut user_cache: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
    for uid in sender_uids {
        if let Ok(user) = state.db_pool.find_user_by_uid(&uid).await {
            user_cache.insert(uid, (user.username.clone(), user.avatar.clone().unwrap_or_default()));
        }
    }

    // 收集所有消息ID用于批量查询已读状态
    let msg_ids: Vec<String> = paginated_messages
        .iter()
        .map(|msg| msg.msg_id.clone())
        .collect();

    // 批量查询消息已读状态
    let mut read_status_map: std::collections::HashMap<String, bool> = std::collections::HashMap::new();
    let mut read_count_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for msg_id in &msg_ids {
        // 检查是否已被当前用户读取
        if let Ok(read_users) = state.db_pool.find_read_users_by_message(msg_id).await {
            read_status_map.insert(msg_id.clone(), read_users.contains(&current_user.uid));
        }

        // 获取消息已读人数
        if let Ok(count) = state.db_pool.get_message_read_count(msg_id).await {
            read_count_map.insert(msg_id.clone(), count as i64);
        }
    }

    // 转换消息格式
    let message_items: Vec<GroupMessageItem> = paginated_messages
        .iter()
        .map(|msg| {
            // 转换消息类型
            let content_type = match msg.msg_type {
                GroupMsgType::Text => "text",
                GroupMsgType::Image => "image",
                GroupMsgType::File => "file",
                GroupMsgType::Voice => "voice",
                GroupMsgType::Video => "video",
                GroupMsgType::Link => "link",
                GroupMsgType::Emoji => "emoji",
                GroupMsgType::Annoucement => "annoucement",
            };

            // 处理时间戳，转为 Unix 时间戳
            let timestamp = msg.send_time
                .map(|dt| dt.timestamp())
                .unwrap_or(0);

            // 处理布尔值
            let is_revoked = msg.is_revoked.unwrap_or(0) == 1;
            let is_announcement = msg.is_announcement.unwrap_or(0) == 1;

            // 处理 mentioned_uids（从JSON转换为Vec<String>）
            let mentioned_uids = if let Some(uids) = &msg.mentioned_uids {
                if let Some(uid_array) = uids.as_array() {
                    uid_array.iter()
                        .filter_map(|v| v.as_str())
                        .map(|s| s.to_string())
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                }
            } else {
                Vec::new()
            };

            // 从缓存中获取发送者信息
            let (sender_name, sender_avatar) = user_cache.get(&msg.sender_uid)
                .cloned()
                .unwrap_or_else(|| (msg.sender_uid.clone(), "".to_string()));

            // 从缓存中获取已读状态和已读人数
            let is_read = read_status_map.get(&msg.msg_id).cloned().unwrap_or(false);
            let read_count = read_count_map.get(&msg.msg_id).cloned().unwrap_or(0);

            GroupMessageItem {
                message_type: "Group".to_string(),
                payload: GroupMessagePayload {
                    message_id: msg.msg_id.clone(),
                    chat_id: msg.gid.clone(),
                    timestamp,
                    sender_id: msg.sender_uid.clone(),
                    sender_name,
                    sender_avatar,
                    receiver_id: msg.gid.clone(),
                    content_type: content_type.to_string(),
                    detail: msg.content.clone(),
                    is_announcement,
                    mentioned_uids,
                    quote_msg_id: msg.quote_msg_id.clone(),
                },
                is_revoked,
                is_read,
                read_count,
            }
        })
        .collect();

    Ok(Json(GroupHistoryResponse {
        total_pages,
        current_page: payload.offset,
        total_items: total_messages,
        messages: message_items,
    }))
}

pub async fn mark_msg_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<ReadRequest>,
) -> AppResult<Json<ReadResponse>> {
    // 获取当前用户信息
    let current_user = state.db_pool.find_user_by_account(&claims.sub).await?;

    // 解析时间戳
    let timestamp = chrono::DateTime::from_timestamp(payload.timestamp, 0)
        .ok_or_else(|| AppError::BadRequest("无效的时间戳".to_string()))?;

    match payload.chat_type.as_str() {
        "private" => {
            // 验证私聊会话
            let private_chat = state.db_pool.find_chat_by_pid(&payload.chat_id).await?;
            let private_chat = match private_chat {
                Some(chat) => chat,
                None => {
                    return Err(AppError::NotFound(format!("私聊会话 {} 不存在", payload.chat_id)));
                }
            };

            // 检查用户是否属于这个私聊
            if current_user.uid != private_chat.uid1 && current_user.uid != private_chat.uid2 {
                return Err(AppError::Forbidden(format!("用户 {} 不是私聊会话 {} 的参与者", current_user.uid, payload.chat_id)));
            }

            // 批量标记私聊消息为已读（只标记对方发送的消息）
            let _affected_rows = state.db_pool.mark_messages_as_read_by_chat_and_time(
                &payload.chat_id,
                &current_user.uid,
                timestamp
            ).await?;
        }
        "group" => {
            // 验证群聊
            let group_chat = match state.db_pool.find_group_by_gid(&payload.chat_id).await? {
                Some(chat) => chat,
                None => {
                    return Err(AppError::NotFound(format!("群聊 {} 不存在", payload.chat_id)));
                }
            };

            // 检查用户是否在群聊中
            let membership = state.db_pool.find_member(&payload.chat_id, &current_user.uid).await?;
            if membership.is_none() {
                return Err(AppError::Forbidden(format!("用户 {} 不是群聊 {} 的成员", current_user.uid, payload.chat_id)));
            }

            // 批量标记群聊消息为已读
            let _affected_rows = state.db_pool.mark_messages_as_read_by_group_and_time(
                &payload.chat_id,
                &current_user.uid,
                timestamp
            ).await?;
        }
        _ => {
            return Err(AppError::BadRequest("无效的聊天类型，必须是 'private' 或 'group'".to_string()));
        }
    }

    Ok(Json(ReadResponse {
        success:true
    }))
}

pub async fn fetch_group_read(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<FetchGroupReadRequest>,
) -> AppResult<Json<FetchGroupReadResponse>> {
    // 通过 account 获取当前用户信息
    let current_user = state.db_pool.find_user_by_account(&claims.sub).await?;

    // 验证群聊是否存在
    let group_chat = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group_chat = match group_chat {
        Some(chat) => chat,
        None => {
            return Err(AppError::NotFound(format!("群聊 {} 不存在", payload.gid)));
        }
    };

    // 检查用户是否在群聊中
    let membership = state.db_pool.find_member(&payload.gid, &current_user.uid).await?;
    if membership.is_none() {
        return Err(AppError::Forbidden(format!("用户 {} 不是群聊 {} 的成员", current_user.uid, payload.gid)));
    }

    // 验证消息ID列表不为空
    if payload.message_ids.is_empty() {
        return Ok(Json(FetchGroupReadResponse {
            read_counts: Vec::new(),
        }));
    }

    // 使用批量查询获取所有消息的已读数量
    let read_count_results = state.db_pool.get_message_read_counts(&payload.message_ids).await?;

    // 将结果转换为 HashMap 以便快速查找
    let read_count_map: std::collections::HashMap<String, i64> = read_count_results.into_iter().collect();

    // 构建响应，确保所有请求的消息ID都有返回值（没有已读记录的为0）
    let read_counts: Vec<GroupReadCountItem> = payload.message_ids
        .iter()
        .map(|msg_id| GroupReadCountItem {
            message_id: msg_id.clone(),
            read_count: read_count_map.get(msg_id).cloned().unwrap_or(0),
        })
        .collect();

    Ok(Json(FetchGroupReadResponse {
        read_counts,
    }))
}