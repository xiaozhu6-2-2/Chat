// use axum::{extract::State, Json};

// use crate::models::{errors::AppResult, responses::ChatListResponse, responses::PrivateChatResponse, responses::GroupChatResponse, requests::PrivateChatRequest, requests::GroupChatRequest, requests::ChatListRequest};
// use crate::state::AppState;

// pub async fn get_chat_list(
//     State(state): State<AppState>,
//     Json(payload): Json<ChatListRequest>,
// ) -> AppResult<Json<ChatListResponse>> {

// }

// pub async fn get_private_chat(
//     State(state): State<AppState>,
//     Json(payload): Json<PrivateChatRequest>,
// ) -> AppResult<Json<PrivateChatResponse>> {

// }

// pub async fn get_group_chat(
//     State(state): State<AppState>,
//     Json(payload): Json<GroupChatRequest>,
// ) -> AppResult<Json<GroupChatResponse>> {

// }