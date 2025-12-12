use axum::Extension;
use axum::{extract::State, Json};

use crate::models::entities::Gender;
use crate::models::others::Claims;
use crate::models::repository::UserRepository;
use crate::models::{errors::AppResult, responses::UserInfoResponse, responses::UserInfoUpdateResponse, responses::FetchProfileResponse, requests::UserInfoUpdateRequest, requests::FetchProfileRequest, entities::GenderOptionExt};
use crate::state::AppState;

pub async fn get_user_info(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<UserInfoResponse>> {
    // 找得到就返回，找不到就报错
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    
    Ok(Json(UserInfoResponse {
        uid: user.uid,
        account: user.account,
        username: user.username,
        gender: user.gender.to_optional_string(),  // 使用新的转换方法
        region: user.region,
        email: user.email,
        create_time: user.create_time.map(|dt| dt.timestamp()),
        avatar: user.avatar,
        bio: user.bio,
    }))
}

pub async fn update_user_info(
    State(state): State<AppState>,
    Extension(cliams): Extension<Claims>,
    Json(payload): Json<UserInfoUpdateRequest>,
) -> AppResult<Json<UserInfoUpdateResponse>> {
    // 找到那个user,然后更新值
    let mut user = state.db_pool.find_user_by_account(&cliams.sub).await?;

    // 更新用户信息
    user.username = payload.username;
    user.gender = Option::<Gender>::from_optional_string(payload.gender);
    user.region = payload.region;
    user.email = payload.email;
    user.avatar = payload.avatar;
    user.bio = payload.bio;

    // 更新数据库
    state.db_pool.save_user(user).await?;

    Ok(Json(UserInfoUpdateResponse { success: true }))
}

pub async fn fetch_user_profile(
    State(state): State<AppState>,
    Json(payload): Json<FetchProfileRequest>,
) -> AppResult<Json<FetchProfileResponse>> {
    // 找到那个user
    let user = state.db_pool.find_user_by_uid(&payload.uid).await?;

    Ok(Json(FetchProfileResponse { 
        uid: user.uid, 
        account: user.account, 
        username: user.username, 
        gender: user.gender.to_optional_string(), 
        region: user.region, 
        email: user.email, 
        avatar: user.avatar, 
        bio: user.bio 
    }))
}