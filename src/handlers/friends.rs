use axum::{extract::State, Json};

use crate::models::{errors::AppResult, responses::SearchUserResponse, responses::FriendProfileResponse, responses::FriendListResponse, responses::FriendRequestResponse, responses::RespondFriendRequestResponse, responses::FriendRequestListResponse, responses::RemoveFriendResponse, responses::UpdateFriendRemarkResponse, responses::UpdateFriendBlacklistResponse, requests::SearchUserRequest, requests::FriendProfileRequest, requests::FriendListRequest, requests::FriendRequestRequest, requests::RespondFriendRequestRequest, requests::FriendRequestListRequest, requests::RemoveFriendRequest, requests::UpdateFriendRemarkRequest, requests::UpdateFriendBlacklistRequest};
use crate::state::AppState;

pub async fn search_user(
    State(state): State<AppState>,
    Json(payload): Json<SearchUserRequest>,
) -> AppResult<Json<SearchUserResponse>> {

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