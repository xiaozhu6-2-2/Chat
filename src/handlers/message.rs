// use axum::{extract::State, Json};

// use crate::models::{errors::AppResult, responses::PrivateHistoryResponse, responses::GroupHistoryResponse, responses::ReadResponse, requests::PrivateHistoryRequest, requests::GroupHistoryRequest, requests::ReadRequest};
// use crate::state::AppState;

// pub async fn get_private_history(
//     State(state): State<AppState>,
//     Json(payload): Json<PrivateHistoryRequest>,
// ) -> AppResult<Json<PrivateHistoryResponse>> {

// }

// pub async fn get_group_history(
//     State(state): State<AppState>,
//     Json(payload): Json<GroupHistoryRequest>,
// ) -> AppResult<Json<GroupHistoryResponse>> {

// }

// pub async fn mark_msg_read(
//     State(state): State<AppState>,
//     Json(payload): Json<ReadRequest>,
// ) -> AppResult<Json<ReadResponse>> {

// }