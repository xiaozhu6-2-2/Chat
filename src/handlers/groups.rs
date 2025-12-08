use axum::Extension;
use axum::{extract::State, Json};

use crate::models::others::Claims;
use crate::models::{errors::{AppResult, AppError}, responses::SearchGroupResponse, responses::GroupProfileResponse, responses::GroupListResponse, requests::SearchGroupRequest, requests::GroupProfileRequest, requests::GroupListRequest};
use crate::models::repository::GroupChatRepository;
use crate::state::AppState;

// pub async fn search_group(
//     State(state): State<AppState>,
//     Json(payload): Json<SearchGroupRequest>,
// ) -> AppResult<Json<SearchGroupResponse>> {

// }

pub async fn get_group_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupProfileRequest>,
) -> AppResult<Json<GroupProfileResponse>> {
    // 首先验证用户是否是该群组的成员
    let member = state.db_pool.find_member(&payload.gid, &claims.sub).await?;

    // 如果用户不是群组成员，返回错误
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: claims.sub.clone(),
            gid: payload.gid.clone(),
        });
    }

    // 获取群组信息
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 构建响应
    Ok(Json(GroupProfileResponse {
        gid: group.gid,
        group_name: group.group_name,
        manager_uid: group.manager_uid,
        avatar: group.group_avatar,
        group_intro: group.group_intro,
        created_at: group.create_time,
    }))
}

// pub async fn get_group_list(
//     State(state): State<AppState>,
//     Json(payload): Json<GroupListRequest>,
// ) -> AppResult<Json<GroupListResponse>> {

// }