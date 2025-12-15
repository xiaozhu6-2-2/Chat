use axum::Extension;
use axum::{extract::{State, Multipart}, Json};

use crate::models::others::Claims;
use crate::models::{errors::AppResult, responses::UploadFileResponse, responses::PreviewFileResponse, responses::DownloadFileResponse, responses::DeleteFileResponse, requests::UploadFileRequest, requests::PreviewFileRequest, requests::DownloadFileRequest, requests::DeleteFileRequest};
use crate::state::AppState;

pub async fn upload_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    multipart: Multipart,
) -> AppResult<Json<UploadFileResponse>> {

}

pub async fn preview_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PreviewFileRequest>,
) -> AppResult<Json<PreviewFileResponse>> {

}

pub async fn download_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DownloadFileRequest>,
) -> AppResult<Json<DownloadFileResponse>> {

}

pub async fn delete_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DeleteFileRequest>,
) -> AppResult<Json<DeleteFileResponse>> {

}