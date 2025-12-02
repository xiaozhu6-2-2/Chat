use axum::{extract::State, Json};

use crate::models::{errors::AppResult, responses::UserInfoResponse, responses::UserInfoUpdateResponse, responses::FetchProfileResponse, requests::UserInfoUpdateRequest, requests::FetchProfileRequest};
use crate::state::AppState;

pub async fn get_user_info() -> AppResult<Json<UserInfoResponse>> {

}

pub async fn update_user_info(
    State(state): State<AppState>,
    Json(payload): Json<UserInfoUpdateRequest>,
) -> AppResult<Json<UserInfoUpdateResponse>> {

}

pub async fn fetch_user_profile(
    State(state): State<AppState>,
    Json(payload): Json<FetchProfileRequest>,
) -> AppResult<Json<FetchProfileResponse>> {

}