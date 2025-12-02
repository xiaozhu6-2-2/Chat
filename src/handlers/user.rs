use axum::Json;

use crate::models::{errors::AppResult, responses::UserInfoResponse};

pub async fn get_user_info() -> AppResult<Json<UserInfoResponse>> {

}