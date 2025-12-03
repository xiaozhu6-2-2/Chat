// use axum::{extract::State, Json};

// use crate::models::{errors::AppResult, responses::SearchGroupResponse, responses::GroupProfileResponse, responses::GroupListResponse, requests::SearchGroupRequest, requests::GroupProfileRequest, requests::GroupListRequest};
// use crate::state::AppState;

// pub async fn search_group(
//     State(state): State<AppState>,
//     Json(payload): Json<SearchGroupRequest>,
// ) -> AppResult<Json<SearchGroupResponse>> {

// }

// pub async fn get_group_profile(
//     State(state): State<AppState>,
//     Json(payload): Json<GroupProfileRequest>,
// ) -> AppResult<Json<GroupProfileResponse>> {

// }

// pub async fn get_group_list(
//     State(state): State<AppState>,
//     Json(payload): Json<GroupListRequest>,
// ) -> AppResult<Json<GroupListResponse>> {

// }