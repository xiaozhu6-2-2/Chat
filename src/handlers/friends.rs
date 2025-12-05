use axum::Extension;
use axum::{extract::State, Json};

use crate::models::entities::GenderOptionExt;
use crate::models::others::Claims;
use crate::models::repository::{UserRepository, FriendshipRepository};
use crate::models::requests::FriendRequestRequest;
use crate::models::responses::{FriendListResponse, FriendRequestResponse};
use crate::models::{errors::AppResult, responses::SearchUserResponse, responses::FriendProfileResponse, requests::SearchUserRequest, requests::FriendProfileRequest};
use crate::state::AppState;

pub async fn search_user(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<SearchUserRequest>,
) -> AppResult<Json<SearchUserResponse>> {
    // 处理分页参数
    let limit = if payload.limit > 0 { payload.limit } else { 20 }; // 默认每页20条
    let offset = if payload.offset >= 0 { payload.offset } else { 0 }; // 默认第0页

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
        created_at: friendship.create_time,
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
                    created_at: *created_at,
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
                    created_at: *created_at,
                    bio: friend_user.bio.clone(),
                    avatar: friend_user.avatar.clone(),
                };
                blacklist.push(friend_item);
            }
        }
    }

    // 7. 构建并返回响应
    let response = FriendListResponse {
        total: (friends.len() + blacklist.len()) as i64,
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
    use crate::models::entities::{ReqStatus, ReqStatusOptionExt};

    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户，获取 sender_uid
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 生成 req_id 使用雪花算法
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let req_id = snowflake.next_id()?.to_string();

    // 4. 创建 FriendRequest 记录
    let friend_request = crate::models::entities::FriendRequest {
        req_id,
        sender_uid: current_user.uid,
        receiver_uid: payload.receiver_id.clone(), // receiver_id 直接就是 uid
        status: ReqStatus::Pending,
        apply_text: Some(payload.message),
        create_time: Some(payload.create_time),
        handle_time: None, // 处理时间设为 NULL
    };

    // 5. 插入数据库
    state.db_pool.save_friend_request(friend_request.clone()).await?;

    // 6. 返回响应
    let response = FriendRequestResponse {
        req_id: friend_request.req_id,
        sender_uid: friend_request.sender_uid,
        receiver_uid: friend_request.receiver_uid,
        apply_text: friend_request.apply_text,
        create_time: friend_request.create_time.unwrap_or_default().to_string(),
        status: Some(ReqStatus::Pending).to_optional_string(),
    };

    Ok(Json(response))
}

// pub async fn respond_friend_request(
//     State(state): State<AppState>,
//     Json(payload): Json<RespondFriendRequestRequest>,
// ) -> AppResult<Json<RespondFriendRequestResponse>> {

// }

// pub async fn get_friend_request_list(
//     State(state): State<AppState>,
//     Json(payload): Json<FriendRequestListRequest>,
// ) -> AppResult<Json<FriendRequestListResponse>> {

// }

// pub async fn remove_friend(
//     State(state): State<AppState>,
//     Json(payload): Json<RemoveFriendRequest>,
// ) -> AppResult<Json<RemoveFriendResponse>> {

// }

// pub async fn update_friend_remark(
//     State(state): State<AppState>,
//     Json(payload): Json<UpdateFriendRemarkRequest>,
// ) -> AppResult<Json<UpdateFriendRemarkResponse>> {

// }

// pub async fn update_friend_blacklist(
//     State(state): State<AppState>,
//     Json(payload): Json<UpdateFriendBlacklistRequest>,
// ) -> AppResult<Json<UpdateFriendBlacklistResponse>> {

// }