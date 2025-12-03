use axum::Extension;
use axum::{extract::State, Json};

use crate::models::entities::GenderOptionExt;
use crate::models::others::Claims;
use crate::models::repository::UserRepository;
use crate::models::{errors::AppResult, responses::SearchUserResponse, responses::FriendProfileResponse, responses::FriendListResponse, responses::FriendRequestResponse, responses::RespondFriendRequestResponse, responses::FriendRequestListResponse, responses::RemoveFriendResponse, responses::UpdateFriendRemarkResponse, responses::UpdateFriendBlacklistResponse, requests::SearchUserRequest, requests::FriendProfileRequest, requests::FriendListRequest, requests::FriendRequestRequest, requests::RespondFriendRequestRequest, requests::FriendRequestListRequest, requests::RemoveFriendRequest, requests::UpdateFriendRemarkRequest, requests::UpdateFriendBlacklistRequest};
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
    Json(payload): Json<FriendProfileRequest>,
) -> AppResult<Json<FriendProfileResponse>> {

}

pub async fn get_friend_list(
    State(state): State<AppState>,
    Json(payload): Json<FriendListRequest>,
) -> AppResult<Json<FriendListResponse>> {

}

pub async fn send_friend_request(
    State(state): State<AppState>,
    Json(payload): Json<FriendRequestRequest>,
) -> AppResult<Json<FriendRequestResponse>> {

}

pub async fn respond_friend_request(
    State(state): State<AppState>,
    Json(payload): Json<RespondFriendRequestRequest>,
) -> AppResult<Json<RespondFriendRequestResponse>> {

}

pub async fn get_friend_request_list(
    State(state): State<AppState>,
    Json(payload): Json<FriendRequestListRequest>,
) -> AppResult<Json<FriendRequestListResponse>> {

}

pub async fn remove_friend(
    State(state): State<AppState>,
    Json(payload): Json<RemoveFriendRequest>,
) -> AppResult<Json<RemoveFriendResponse>> {

}

pub async fn update_friend_remark(
    State(state): State<AppState>,
    Json(payload): Json<UpdateFriendRemarkRequest>,
) -> AppResult<Json<UpdateFriendRemarkResponse>> {

}

pub async fn update_friend_blacklist(
    State(state): State<AppState>,
    Json(payload): Json<UpdateFriendBlacklistRequest>,
) -> AppResult<Json<UpdateFriendBlacklistResponse>> {

}