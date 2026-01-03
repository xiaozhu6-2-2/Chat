use axum::Extension;
use axum::{extract::State, Json};

use crate::models::entities::{Gender, OptionalEnumExt, AssociationType, AccessLevel, AccessTarget};
use crate::models::others::Claims;
use crate::models::repository::{UserRepository, FileRepository};
use crate::models::requests::UserAvatarRequest;
use crate::models::responses::{UserAvatarResponse, UserTokenResponse};
use crate::models::{errors::AppResult, responses::UserInfoResponse, responses::UserInfoUpdateResponse, responses::FetchProfileResponse, requests::UserInfoUpdateRequest, requests::FetchProfileRequest};
use crate::state::AppState;

pub async fn validate(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<UserTokenResponse>> {
    // 验证用户是否仍然存在
    let _user = state.db_pool.find_user_by_account(&claims.sub).await?;

    Ok(Json(UserTokenResponse {
        valid: true,
    }))
}

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

pub async fn update_user_avatar(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<UserAvatarRequest>
) -> AppResult<Json<UserAvatarResponse>> {
    // 1. 验证输入参数
    if payload.file_id.is_empty() {
        return Err(crate::models::errors::AppError::BadRequest("file_id 不能为空".to_string()));
    }

    // 2. 获取用户信息（从claims.sub获取用户账号）
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    let user_uid = user.uid.clone();

    // 3. 验证文件是否存在
    let metadata = state.db_pool.find_file_metadata_by_id(&payload.file_id).await?;
    let file_meta = metadata.ok_or_else(|| crate::models::errors::AppError::NotFound("文件不存在".to_string()))?;

    // 4. 验证文件类型必须是图片
    if !file_meta.file_type.eq_ignore_ascii_case("image") {
        return Err(crate::models::errors::AppError::BadRequest("头像必须是图片类型".to_string()));
    }

    // 5. 验证用户对文件的访问权限（至少需要View权限）
    let has_permission = state.db_pool.verify_file_permission(
        &payload.file_id,
        &user_uid,
        AccessLevel::View
    ).await?;

    if !has_permission {
        return Err(crate::models::errors::AppError::Forbidden("没有权限访问该文件".to_string()));
    }

    // 6. 处理旧头像文件（如果存在）
    if let Some(old_avatar_file_id) = &user.avatar {
        // 软删除旧头像文件
        let _ = state.db_pool.soft_delete_file(old_avatar_file_id).await;
    }

    // 删除旧的头像文件关联（如果有）
    let _ = state.db_pool.batch_delete_associations_by_target(
        AssociationType::UserAvatar,
        &user_uid
    ).await;

    // 7. 创建新的UserAvatar类型的文件关联
    state.db_pool.create_file_association(
        &payload.file_id,
        AssociationType::UserAvatar,
        &user_uid,  // 关联到用户自身
        &user_uid,  // 创建者是当前用户
    ).await?;

    // 8. 为所有人授予头像文件的下载权限（target_id为None表示所有人可见）
    state.db_pool.grant_file_permission(
        &payload.file_id,
        AccessTarget::Public,
        None,  // None表示所有人可见
        AccessLevel::Download,
        &user_uid,
        None,  // 永不过期
    ).await?;

    // 9. 更新用户表的avatar字段（存储file_id）
    let mut updated_user = user;
    updated_user.avatar = Some(payload.file_id.clone());
    state.db_pool.save_user(updated_user).await?;

    // 10. 返回成功响应
    Ok(Json(UserAvatarResponse { success: true }))
}