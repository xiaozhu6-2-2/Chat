use axum::Extension;
use axum::{extract::State, Json};

use crate::models::entities::{OptionalEnumExt, ReqStatus, Friends, PrivateChat};
use crate::models::others::Claims;
use crate::models::repository::{UserRepository, FriendshipRepository};
use crate::models::requests::{FriendRequestRequest, RemoveFriendRequest, RespondFriendRequestRequest, UpdateFriendRemarkBlacklistGroupByRequest};
use crate::models::responses::{FriendListResponse, FriendRequestItem, FriendRequestListResponse, FriendRequestResponse, RemoveFriendResponse, RespondFriendRequestResponse, UpdateFriendRemarkBlacklistGroupByResponse};
use crate::models::{errors::AppResult, responses::SearchUserResponse, responses::FriendProfileResponse, requests::SearchUserRequest, requests::FriendProfileRequest};
use crate::state::AppState;

pub async fn search_user(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<SearchUserRequest>,
) -> AppResult<Json<SearchUserResponse>> {
    // 验证分页参数
    if payload.limit < 0 {
        return Err(crate::models::errors::AppError::BadRequest(
            "Limit cannot be negative".to_string()
        ));
    }

    if payload.offset < 0 {
        return Err(crate::models::errors::AppError::BadRequest(
            "Offset cannot be negative".to_string()
        ));
    }

    // 处理分页参数
    let limit = if payload.limit > 0 { payload.limit } else { 20 }; // 默认每页20条
    let offset = payload.offset; // 默认第0页

    let search_results = if payload.query.is_empty() {
        // 如果查询为空，返回空结果
        Vec::new()
    } else {
        // 使用多种搜索方式组合查询
        let mut all_results = Vec::new();
        let mut seen_uids = std::collections::HashSet::new();

        // 1. UID 精准搜索（如果查询是数字且长度合适）
        let is_numeric_query = payload.query.chars().all(|c| c.is_ascii_digit());
        if is_numeric_query && payload.query.len() > 8 {
            if let Ok(user) = state.db_pool.find_user_by_uid(&payload.query).await {
                seen_uids.insert(user.uid.clone());
                all_results.push(user);
            }
        }

        // 2. Account 精准搜索（如果查询是字母数字组合且长度合适）
        let is_alphanumeric_query = payload.query.chars().all(|c| c.is_ascii_alphanumeric());
        if is_alphanumeric_query && payload.query.len() >= 3 {
            if let Ok(user) = state.db_pool.find_user_by_account(&payload.query).await {
                // 避免重复添加相同用户
                if !seen_uids.contains(&user.uid) {
                    seen_uids.insert(user.uid.clone());
                    all_results.push(user);
                }
            }
        }

        // 3. 用户名模糊搜索（始终执行）
        if let Ok(mut username_results) = state.db_pool.find_user_by_username(&payload.query).await {
            // 过滤掉已经通过 UID 或 Account 搜索找到的用户，避免重复
            username_results.retain(|user| !seen_uids.contains(&user.uid));
            all_results.extend(username_results);
        }

        all_results
    };

    // 实现分页
    let total_users = search_results.len() as i64;

    // 计算总页数（向上取整）
    let total_pages = if total_users == 0 {
        0
    } else {
        (total_users / limit) + 1
    };

    // 检查页码是否超出范围
    if total_pages > 0 && offset >= total_pages {
        return Err(crate::models::errors::AppError::PageOutOfRange {
            page: offset,
            total_pages,
        });
    }

    let start_index = (offset * limit) as usize;
    let end_index = std::cmp::min(start_index + limit as usize, total_users as usize);

    let paginated_users = if start_index < total_users as usize {
        search_results[start_index..end_index].to_vec()// 从start_index到end_index-1
    } else {
        Vec::new()
    };

    // 转换为响应格式
    let response_users: Vec<crate::models::responses::SearchUserItem> = paginated_users
        .into_iter()
        .map(|user| crate::models::responses::SearchUserItem {
            uid: user.uid,
            username: user.username,
            gender: user.gender.to_optional_string(),
            avatar: user.avatar,
            bio: user.bio,
        })
        .collect();

    Ok(Json(SearchUserResponse {
        total_pages,
        current_page: offset,
        total_items: total_users,
        users: response_users,
    }))
}

pub async fn get_friend_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<FriendProfileRequest>,
) -> AppResult<Json<FriendProfileResponse>> {
    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 通过两个 uid 查找好友关系
    let friendship = state.db_pool
        .find_friendship_by_users(&current_user.uid, &payload.uid)
        .await?;

    // 检查是否为好友
    let friendship = friendship.ok_or(crate::models::errors::AppError::NotFound(
        "Friendship not found".to_string()
    ))?;

    // 4. 通过 payload.uid 查找好友用户资料
    let friend_user = state.db_pool.find_user_by_uid(&payload.uid).await?;

    // 5. 判断当前用户是 uid 还是 to_uid，以确定使用哪个备注和分组
    let (remark, group_by, is_blacklisted) = if current_user.uid == friendship.uid {
        (
            friendship.remark.unwrap_or_default(),
            friendship.group_by.unwrap_or_default(),
            friendship.is_blacklist.unwrap_or(0) == 1
        )
    } else {
        (
            friendship.to_remark.unwrap_or_default(),
            friendship.to_group_by.unwrap_or_default(),
            friendship.to_is_blacklist.unwrap_or(0) == 1
        )
    };

    // 6. 构建 FriendProfileResponse
    let response = FriendProfileResponse {
        fid: friendship.fid,
        uid: friend_user.uid,
        account: friend_user.account,
        username: friend_user.username,
        remark,
        group_by,
        is_blacklisted,
        created_at: friendship.create_time.map(|dt| dt.timestamp()),
        bio: friend_user.bio,
        avatar: friend_user.avatar,
        gender: friend_user.gender.to_optional_string(),
        region: friend_user.region,
        email: friend_user.email,
    };

    Ok(Json(response))
}

pub async fn get_friend_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> AppResult<Json<FriendListResponse>> {
    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 根据当前用户的 uid 获取所有好友关系
    let friendships = state.db_pool.find_friendship_by_uid(&current_user.uid).await?;

    // 4. 收集所有好友的 uid，区分普通好友和黑名单好友
    let mut normal_friend_uids = Vec::new();
    let mut blacklist_friend_uids = Vec::new();
    let mut friendship_map = std::collections::HashMap::new();

    for friendship in friendships {
        // 判断当前用户是 uid 还是 to_uid
        let (friend_uid, remark, group_by, is_blacklisted) = if current_user.uid == friendship.uid {
            (
                friendship.to_uid.clone(),
                friendship.remark.clone().unwrap_or_default(),
                friendship.group_by.clone().unwrap_or_default(),
                friendship.is_blacklist.unwrap_or(0) == 1
            )
        } else {
            (
                friendship.uid.clone(),
                friendship.to_remark.clone().unwrap_or_default(),
                friendship.to_group_by.clone().unwrap_or_default(),
                friendship.to_is_blacklist.unwrap_or(0) == 1
            )
        };

        // 根据 is_blacklisted 分别添加到不同的列表
        if is_blacklisted {
            blacklist_friend_uids.push(friend_uid.clone());
        } else {
            normal_friend_uids.push(friend_uid.clone());
        }

        // 将好友关系信息存入 map
        friendship_map.insert(friend_uid, (friendship.fid, remark, group_by, friendship.create_time, is_blacklisted));
    }

    // 5. 查找普通好友的详细信息
    let mut friends = Vec::new();
    for friend_uid in normal_friend_uids {
        // 根据 uid 查找好友用户信息
        if let Ok(friend_user) = state.db_pool.find_user_by_uid(&friend_uid).await {
            // 从 map 中获取好友关系信息
            if let Some((fid, remark, group_by, created_at, _)) = friendship_map.get(&friend_uid) {
                let friend_item = crate::models::responses::FriendItem {
                    fid: fid.clone(),
                    uid: friend_user.uid.clone(),
                    username: friend_user.username.clone(),
                    remark: remark.clone(),
                    group_by: group_by.clone(),
                    is_blacklisted: false, // 普通好友
                    created_at: created_at.map(|dt| dt.timestamp()),
                    bio: friend_user.bio.clone(),
                    avatar: friend_user.avatar.clone(),
                };
                friends.push(friend_item);
            }
        }
    }

    // 6. 查找黑名单好友的详细信息
    let mut blacklist = Vec::new();
    for friend_uid in blacklist_friend_uids {
        // 根据 uid 查找好友用户信息
        if let Ok(friend_user) = state.db_pool.find_user_by_uid(&friend_uid).await {
            // 从 map 中获取好友关系信息
            if let Some((fid, remark, group_by, created_at, _)) = friendship_map.get(&friend_uid) {
                let friend_item = crate::models::responses::FriendItem {
                    fid: fid.clone(),
                    uid: friend_user.uid.clone(),
                    username: friend_user.username.clone(),
                    remark: remark.clone(),
                    group_by: group_by.clone(),
                    is_blacklisted: true, // 黑名单好友
                    created_at: created_at.map(|dt| dt.timestamp()),
                    bio: friend_user.bio.clone(),
                    avatar: friend_user.avatar.clone(),
                };
                blacklist.push(friend_item);
            }
        }
    }

    // 7. 构建并返回响应
    let response = FriendListResponse {
        friends,
        blacklist,
    };

    Ok(Json(response))
}

pub async fn send_friend_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<FriendRequestRequest>,
) -> AppResult<Json<FriendRequestResponse>> {

    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户，获取 sender_uid
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 检查是否尝试添加自己为好友
    if current_user.uid == payload.receiver_id {
        return Err(crate::models::errors::AppError::BadRequest(
            "Cannot send friend request to yourself".to_string()
        ));
    }

    // 4. 检查两人是否已经是好友关系
    let existing_friendship = state.db_pool.find_friendship_by_users(
        &current_user.uid,
        &payload.receiver_id
    ).await?;

    if existing_friendship.is_some() {
        return Err(crate::models::errors::AppError::BadRequest(
            "Users are already friends".to_string()
        ));
    }

    // 5. 检查是否已有待处理的好友申请
    let pending_requests_from_sender = state.db_pool.find_friend_request_by_sender(
        &current_user.uid
    ).await?;

    let existing_pending = pending_requests_from_sender.iter().any(|req| {
        req.receiver_uid == payload.receiver_id && matches!(req.status, ReqStatus::Pending)
    });

    if existing_pending {
        return Err(crate::models::errors::AppError::BadRequest(
            "Friend request already sent and pending".to_string()
        ));
    }

    // 6. 生成 req_id 使用雪花算法
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let req_id = snowflake.next_id()?.to_string();

    // 7. 创建 FriendRequest 记录
    let friend_request = crate::models::entities::FriendRequest {
        req_id,
        sender_uid: current_user.uid,
        receiver_uid: payload.receiver_id.clone(), // receiver_id 直接就是 uid
        status: ReqStatus::Pending,
        apply_text: Some(payload.message),
        create_time: None,  // 让数据库使用 DEFAULT CURRENT_TIMESTAMP
        handle_time: None, // 处理时间设为 NULL
    };

    // 8. 插入数据库
    state.db_pool.save_friend_request(friend_request.clone()).await?;

    // 9. 获取接收者信息
    let receiver_user = state.db_pool.find_user_by_uid(&friend_request.receiver_uid).await?;

    // 10. 返回响应
    let response = FriendRequestResponse {
        req_id: friend_request.req_id,
        sender_uid: friend_request.sender_uid,
        receiver_uid: friend_request.receiver_uid,
        receiver_name: receiver_user.username,
        receiver_avatar: receiver_user.avatar,
        apply_text: friend_request.apply_text,
        create_time: friend_request.create_time.map(|dt| dt.timestamp()).unwrap_or(0),
        status: Some(ReqStatus::Pending).to_optional_string(),
    };

    Ok(Json(response))
}

pub async fn respond_friend_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RespondFriendRequestRequest>,
) -> AppResult<Json<RespondFriendRequestResponse>> {

    // 1. 从 claims 中提取当前用户的 account
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 根据 req_id 查找好友请求记录
    let friend_request = state.db_pool.find_friend_request_by_id(&payload.req_id).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound(
            format!("Friend request {} not found", payload.req_id)
        ))?;

    // 4. 检查当前用户是否是请求的接收者
    if friend_request.receiver_uid != current_user.uid {
        return Err(crate::models::errors::AppError::BadRequest(
            "You are not authorized to respond to this friend request".to_string()
        ));
    }

    // 5. 检查请求状态
    match friend_request.status {
        ReqStatus::Accepted => {
            return Err(crate::models::errors::AppError::BadRequest(
                "Friend request has already been accepted".to_string()
            ));
        }
        ReqStatus::Rejected => {
            return Err(crate::models::errors::AppError::BadRequest(
                "Friend request has already been rejected".to_string()
            ));
        }
        ReqStatus::Expired => {
            return Err(crate::models::errors::AppError::BadRequest(
                "Friend request has expired".to_string()
            ));
        }
        ReqStatus::Pending => {
            // 继续处理
        }
    }

    // 6. 生成 handle_time（使用服务器时间）
    let handle_time = chrono::Utc::now();

    // 7. 根据 action 处理请求
    match payload.action.as_str() {
        "accept" => {
            // 检查两人是否已经是好友
            let existing_friendship = state.db_pool.find_friendship_by_users(
                &friend_request.sender_uid,
                &friend_request.receiver_uid
            ).await?;

            if existing_friendship.is_some() {
                return Err(crate::models::errors::AppError::BadRequest(
                    "Users are already friends".to_string()
                ));
            }

            // 生成统一的ID（同时用于fid和pid）
            let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
            let id = snowflake.next_id()?.to_string();
            let fid = id.clone();
            let pid = id;

            // 确保较小的 uid 在前，较大的 uid 在后
            let (uid, to_uid) = if friend_request.sender_uid < friend_request.receiver_uid {
                (friend_request.sender_uid.clone(), friend_request.receiver_uid.clone())
            } else {
                (friend_request.receiver_uid.clone(), friend_request.sender_uid.clone())
            };

            // 创建好友关系实体
            let friendship = Friends {
                fid: fid.clone(),
                uid: uid.clone(),
                to_uid: to_uid.clone(),
                create_time: None, // 让数据库自动设置
                is_blacklist: Some(0),
                to_is_blacklist: Some(0),
                remark: None,
                to_remark: None,
                group_by: None,
                to_group_by: None,
            };

            // 创建私聊会话实体
            let private_chat = PrivateChat {
                pid: pid.clone(),
                uid1: uid.clone(),
                uid2: to_uid.clone(),
                create_time: None, // 让数据库自动设置
                is_pinned_by_uid1: Some(0),
                is_pinned_by_uid2: Some(0),
                do_not_disturb_uid1: Some(0),
                do_not_disturb_uid2: Some(0),
            };

            // 调用 Repository 方法处理所有操作
            state.db_pool.accept_friend_request_with_chat(
                &friend_request.req_id,
                handle_time,
                friendship,
                private_chat
            ).await?;

            // 返回成功响应，包含另一位用户的 ID 和好友关系 ID
            Ok(Json(RespondFriendRequestResponse {
                uid: friend_request.sender_uid,
                fid,
                pid
            }))
        }
        "reject" => {
            // 更新好友请求状态为已拒绝
            state.db_pool.update_request_status(
                &payload.req_id,
                "rejected",
                handle_time
            ).await?;

            // 返回拒绝响应，三个字段都返回 "Rejected"
            Ok(Json(RespondFriendRequestResponse {
                uid: "Rejected".to_string(),
                fid: "Rejected".to_string(),
                pid: "Rejected".to_string(),
            }))
        }
        _ => {
            Err(crate::models::errors::AppError::BadRequest(
                "Invalid action. Must be 'accept' or 'reject'".to_string()
            ))
        }
    }
}

pub async fn get_friend_request_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<FriendRequestListResponse>> {
    // 1. 从 claims 中提取当前用户的 account
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 查找用户发送的好友请求
    let sent_requests = state.db_pool.find_friend_request_by_sender(&current_user.uid).await?;

    // 4. 查找用户接收的好友请求
    let received_requests = state.db_pool.find_friend_request_by_receiver(&current_user.uid).await?;

    // 5. 转换发送的请求为响应格式
    let mut requests = Vec::new();
    for req in sent_requests {
        // 查询接收者信息
        let receiver = state.db_pool.find_user_by_uid(&req.receiver_uid).await
            .map_err(|e| crate::models::errors::AppError::InternalError(
                format!("Failed to fetch receiver {}: {}", req.receiver_uid, e)
            ))?;

        let request_item = FriendRequestItem {
            req_id: req.req_id,
            sender_uid: req.sender_uid,
            sender_name: current_user.username.clone(),
            sender_avatar: current_user.avatar.clone().unwrap_or_default(),
            receiver_uid: req.receiver_uid,
            receiver_name: receiver.username,
            receiver_avatar: receiver.avatar.unwrap_or_default(),
            apply_text: req.apply_text,
            create_time: req.create_time.map(|dt| dt.timestamp()),
            status: req.status.to_string(),
        };
        requests.push(request_item);
    }

    // 6. 转换接收的请求为响应格式
    let mut receives = Vec::new();
    for req in received_requests {
        // 查询发送者信息
        let sender = state.db_pool.find_user_by_uid(&req.sender_uid).await
            .map_err(|e| crate::models::errors::AppError::InternalError(
                format!("Failed to fetch sender {}: {}", req.sender_uid, e)
            ))?;

        let request_item = FriendRequestItem {
            req_id: req.req_id,
            sender_uid: req.sender_uid,
            sender_name: sender.username,
            sender_avatar: sender.avatar.unwrap_or_default(),
            receiver_uid: req.receiver_uid,
            receiver_name: current_user.username.clone(),
            receiver_avatar: current_user.avatar.clone().unwrap_or_default(),
            apply_text: req.apply_text,
            create_time: req.create_time.map(|dt| dt.timestamp()),
            status: req.status.to_string(),
        };
        receives.push(request_item);
    }

    // 7. 计算总数
    let total = (requests.len() + receives.len()) as i64;

    // 8. 返回响应
    Ok(Json(FriendRequestListResponse {
        total,
        requests,
        receives,
    }))
}

pub async fn remove_friend(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RemoveFriendRequest>,
) -> AppResult<Json<RemoveFriendResponse>> {
    // 1. 从 claims 中提取当前用户的 account
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 根据 fid 查找好友关系
    let friendship = state.db_pool.find_friendship_by_fid(&payload.fid).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound(
            format!("Friendship {} not found", payload.fid)
        ))?;

    // 4. 检查当前用户是否是好友关系中的一方
    if friendship.uid != current_user.uid && friendship.to_uid != current_user.uid {
        return Err(crate::models::errors::AppError::BadRequest(
            "You are not authorized to delete this friendship".to_string()
        ));
    }

    // 5. 删除好友关系及其相关的私聊会话和消息
    state.db_pool.delete_friendship_with_chat(&payload.fid).await?;

    // 6. 返回成功响应
    Ok(Json(RemoveFriendResponse { 
        success: true 
    }))
}

pub async fn update_friend_remark_blacklist_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UpdateFriendRemarkBlacklistGroupByRequest>,
) -> AppResult<Json<UpdateFriendRemarkBlacklistGroupByResponse>> {
    // 1. 从 claims 中提取当前用户的 account
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 根据 fid 查找好友关系
    let friendship = state.db_pool.find_friendship_by_fid(&payload.fid).await?
        .ok_or_else(|| crate::models::errors::AppError::NotFound(
            format!("Friendship {} not found", payload.fid)
        ))?;

    // 4. 检查当前用户是否是好友关系中的一方
    if friendship.uid != current_user.uid && friendship.to_uid != current_user.uid {
        return Err(crate::models::errors::AppError::BadRequest(
            "You are not authorized to update this friendship".to_string()
        ));
    }

    // 5. 判断当前用户是 uid 还是 to_uid，以便更新正确的字段
    let is_current_user_uid = friendship.uid == current_user.uid;

    // 6. 构建更新后的好友关系
    let mut updated_friendship = friendship.clone();

    if is_current_user_uid {
        // 当前用户是 uid，更新对应的字段
        updated_friendship.remark = payload.remark;
        updated_friendship.is_blacklist = Some(if payload.is_blacklisted { 1 } else { 0 });
        updated_friendship.group_by = payload.group_by;
    } else {
        // 当前用户是 to_uid，更新对应的字段
        updated_friendship.to_remark = payload.remark;
        updated_friendship.to_is_blacklist = Some(if payload.is_blacklisted { 1 } else { 0 });
        updated_friendship.to_group_by = payload.group_by;
    }

    // 7. 保存更新后的好友关系
    state.db_pool.save_friendship(updated_friendship).await?;

    // 8. 返回成功响应
    Ok(Json(UpdateFriendRemarkBlacklistGroupByResponse { 
        success: true 
    }))
}