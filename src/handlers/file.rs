use axum::Extension;
use axum::{extract::{State, Multipart}, Json, response::Response};

use crate::models::others::Claims;
use crate::models::{errors::AppResult, errors::AppError, responses::UploadFileResponse, responses::PreviewFileResponse, responses::DeleteFileResponse, requests::PreviewFileRequest, requests::DownloadFileRequest, requests::DeleteFileRequest};
use crate::models::entities::{AccessTarget, AccessLevel, FileStatus};
use crate::state::AppState;
use crate::utils::{snowflake::Snowflake, file_utils};
use crate::models::repository::{UserRepository, FileRepository};

pub async fn upload_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    mut multipart: Multipart,
) -> AppResult<Json<UploadFileResponse>> {
    // 1. 解析Multipart数据
    let mut file_data: Option<Vec<u8>> = None;
    let mut file_name: Option<String> = None;
    let mut file_type: Option<String> = None;

    while let Some(mut field) = multipart.next_field().await.map_err(|e| {
        AppError::BadRequest(format!("Failed to read multipart field: {}", e))
    })? {
        let field_name = field.name().unwrap_or("").to_string();

        match field_name.as_str() {
            "file" => {
                let mut data = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read file chunk: {}", e))
                })? {
                    data.extend_from_slice(&chunk);
                }
                file_data = Some(data);
            },
            "fileName" => {
                file_name = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read fileName: {}", e))
                })?);
            },
            "fileType" => {
                file_type = Some(field.text().await.map_err(|e| {
                    AppError::BadRequest(format!("Failed to read fileType: {}", e))
                })?);
            },
            _ => {
                return Err(AppError::BadRequest(format!("Unknown field: {}", field_name)));
            }
        }
    }

    // 2. 验证必填字段
    let file_data = file_data.ok_or_else(|| AppError::BadRequest("File field is required".to_string()))?;
    let file_name = file_name.ok_or_else(|| AppError::BadRequest("fileName field is required".to_string()))?;
    let file_type = file_type.ok_or_else(|| AppError::BadRequest("fileType field is required".to_string()))?;

    // 3. 获取用户信息（从Claims的sub获取用户uid）
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    let user_uid = user.uid;

    // 4. 文件大小验证
    let max_size = file_utils::get_max_file_size();
    if file_data.len() > max_size {
        return Err(AppError::FileTooLarge(
            format!("File size exceeds maximum allowed size ({} MB)", max_size / 1024 / 1024)
        ));
    }

    // 5. MIME类型检测和验证
    let detected_mime = file_utils::detect_mime_type(&file_name, &file_data);
    file_utils::validate_file_type(&detected_mime, &file_type)?;

    // 6. 计算文件哈希
    let file_hash = file_utils::calculate_sha256(&file_data);

    // 7. 文件存储处理
    let storage_path_result = file_utils::determine_storage_path(&file_hash, &detected_mime);

    // 查询是否已存在相同哈希的文件
    let existing_storage = state.db_pool.find_file_storage_by_hash(&file_hash).await?;

    let storage = if let Some(storage) = existing_storage {
        // 文件已存在，增加引用计数
        state.db_pool.increment_reference_count(&storage.storage_id).await?;
        storage
    } else {
        // 新文件，保存到磁盘
        let storage_path = storage_path_result?;
        file_utils::save_file_to_disk(&storage_path, &file_data).await?;

        // 创建存储记录
        state.db_pool.create_or_get_file_storage(
            &file_hash,
            &storage_path,
            None, // thumbnail_path - 可以后续生成
            file_data.len() as i64,
            &detected_mime,
        ).await?
    };

    // 8. 创建文件元数据
    let snowflake = Snowflake::new(1, None)?;
    let file_id = snowflake.next_id()?.to_string();

    // 获取文件扩展名
    let extension = file_utils::get_extension_from_mime(&detected_mime);

    state.db_pool.create_file_metadata(
        &file_id,
        &storage.storage_id,
        &user_uid,
        &format!("{}.{}", file_hash, extension), // original_name 使用哈希名
        &file_name, // display_name 使用用户提供的名称
        &file_type,
    ).await?;

    // 9. 创建权限记录（上传者获得管理权限）
    state.db_pool.grant_file_permission(
        &file_id,
        AccessTarget::User,
        Some(user_uid.clone()),
        AccessLevel::Manage,
        &user_uid,
        None, // 永不过期
    ).await?;

    // 10. 查询文件元数据以获取准确的 upload_time
    let metadata = state.db_pool.find_file_metadata_by_id(&file_id).await?;
    let meta = metadata.ok_or_else(|| AppError::InternalError("Failed to retrieve file metadata".to_string()))?;

    // 将数据库的 DateTime<Utc> 转换为 i64 时间戳
    let upload_time = meta.upload_time
        .map(|dt| dt.timestamp())
        .unwrap_or(0);

    // 11. 构造响应
    let response = UploadFileResponse {
        file_id,
        display_name: file_name,
        file_size: file_data.len() as i64,
        mime_type: detected_mime,
        file_type,
        upload_time, // 使用数据库查询到的准确时间戳
        owner_uid: user_uid,
    };

    Ok(Json(response))
}

pub async fn preview_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<PreviewFileRequest>,
) -> AppResult<Json<PreviewFileResponse>> {
    // 1. 验证输入参数
    if payload.file_id.is_empty() {
        return Err(AppError::BadRequest("file_id 不能为空".to_string()));
    }

    // 获取用户信息
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    let user_uid = user.uid;

    // 2. 验证用户对文件的查看权限
    let has_permission = state.db_pool.verify_file_permission(
        &payload.file_id,
        &user_uid,
        AccessLevel::View
    ).await?;

    if !has_permission {
        return Err(AppError::Forbidden("没有权限访问该文件".to_string()));
    }

    // 3. 获取文件元数据
    let metadata = state.db_pool.find_file_metadata_by_id(&payload.file_id).await?;
    let meta = metadata.ok_or_else(|| AppError::NotFound("文件不存在".to_string()))?;

    // 4. 获取文件存储信息以获取文件大小
    let storage = state.db_pool.find_file_storage_by_id(&meta.storage_id).await?;
    let storage_info = storage.ok_or_else(|| {
        AppError::NotFound("文件存储信息不存在".to_string())
    })?;

    // 5. 更新文件的最后访问时间
    state.db_pool.update_last_access_time(&payload.file_id).await?;

    // 6. 构造并返回响应
    let response = PreviewFileResponse {
        display_name: meta.display_name,
        file_size: storage_info.file_size.unwrap_or(0),
        file_type: meta.file_type,
    };

    Ok(Json(response))
}

pub async fn download_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DownloadFileRequest>,
) -> AppResult<Response> {
    // 1. 获取用户信息
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    let user_uid = user.uid;

    // 2. 验证用户对文件的下载权限
    let has_permission = state.db_pool.verify_file_permission(
        &payload.file_id,
        &user_uid,
        AccessLevel::Download
    ).await?;

    if !has_permission {
        return Err(AppError::Forbidden("没有下载权限".to_string()));
    }

    // 3. 获取文件元数据
    let metadata = state.db_pool.find_file_metadata_by_id(&payload.file_id).await?;
    let meta = metadata.ok_or_else(|| AppError::NotFound("文件不存在".to_string()))?;

    // 4. 获取文件存储信息
    let storage = state.db_pool.find_file_storage_by_id(&meta.storage_id).await?;
    let storage_info = storage.ok_or_else(|| AppError::NotFound("文件存储信息不存在".to_string()))?;

    // 5. 检查文件状态
    if meta.file_status != FileStatus::Active {
        return Err(AppError::BadRequest("文件不可用".to_string()));
    }

    // 6. 读取文件内容
    let file_content = file_utils::read_file_from_disk(&storage_info.file_path).await?;

    // 7. 创建下载响应
    let response = file_utils::create_download_response(
        file_content,
        &meta.display_name,
        &storage_info.mime_type,
    );

    Ok(response)
}

pub async fn delete_file(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<DeleteFileRequest>,
) -> AppResult<Json<DeleteFileResponse>> {
    // 1. 获取用户信息
    let user = state.db_pool.find_user_by_account(&claims.sub).await?;
    let user_uid = user.uid;

    // 2. 验证文件存在
    let metadata = state.db_pool.find_file_metadata_by_id(&payload.file_id).await?;
    let meta = metadata.ok_or_else(|| AppError::NotFound("文件不存在".to_string()))?;

    // 3. 检查文件状态（不能删除已删除的文件）
    if meta.file_status == FileStatus::Deleted {
        return Err(AppError::BadRequest("文件已被删除".to_string()));
    }

    // 4. 验证用户对文件的管理权限
    let has_permission = state.db_pool.verify_file_permission(
        &payload.file_id,
        &user_uid,
        AccessLevel::Manage
    ).await?;

    if !has_permission {
        return Err(AppError::Forbidden("没有删除权限".to_string()));
    }

    // 5. 执行软删除
    state.db_pool.soft_delete_file(&payload.file_id).await?;

    // 6. 返回成功响应
    Ok(Json(DeleteFileResponse { success: true }))
}