use sha2::{Sha256, Digest};
use mime_guess;
use infer;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use crate::models::errors::{AppError, AppResult};

/// 根据哈希和MIME类型确定存储路径
pub fn determine_storage_path(file_hash: &str, mime_type: &str) -> AppResult<String> {
    // 根据哈希前两位创建子目录，避免单个目录文件过多
    let subdir = if file_hash.len() >= 2 {
        &file_hash[0..2]
    } else {
        "00"
    };

    // 根据MIME类型获取文件扩展名
    let extension = get_extension_from_mime(mime_type);
    let filename = format!("{}.{}", file_hash, extension);

    Ok(format!("{}/{}", subdir, filename))
}

/// 异步保存文件到磁盘
pub async fn save_file_to_disk(path: &str, data: &[u8]) -> AppResult<()> {
    use tokio::fs;

    // 构建完整路径
    let base_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    let full_path = format!("{}/{}", base_dir, path);

    // 确保父目录存在
    if let Some(parent) = std::path::Path::new(&full_path).parent() {
        fs::create_dir_all(parent).await.map_err(|e| {
            AppError::FileStorageError(format!("Failed to create directory: {}", e))
        })?;
    }

    // 写入文件
    fs::write(&full_path, data).await.map_err(|e| {
        AppError::FileStorageError(format!("Failed to write file: {}", e))
    })?;

    Ok(())
}

/// 根据MIME类型获取文件扩展名
pub fn get_extension_from_mime(mime_type: &str) -> &str {
    match mime_type {
        // 图片类型
        "image/jpeg" => "jpg",
        "image/jpg" => "jpg",
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/bmp" => "bmp",
        "image/tiff" => "tiff",
        "image/ico" => "ico",
        "image/x-icon" => "ico",

        // 视频类型
        "video/mp4" => "mp4",
        "video/avi" => "avi",
        "video/mov" => "mov",
        "video/wmv" => "wmv",
        "video/flv" => "flv",
        "video/webm" => "webm",
        "video/mkv" => "mkv",
        "video/quicktime" => "mov",
        "video/x-msvideo" => "avi",

        // 音频类型
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/aac" => "aac",
        "audio/flac" => "flac",
        "audio/mp4" => "m4a",

        // 文档类型
        "application/pdf" => "pdf",
        "application/msword" => "doc",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => "docx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-powerpoint" => "ppt",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => "pptx",
        "text/plain" => "txt",
        "text/rtf" => "rtf",
        "text/csv" => "csv",

        // 压缩文件
        "application/zip" => "zip",
        "application/x-rar-compressed" => "rar",
        "application/x-7z-compressed" => "7z",
        "application/x-tar" => "tar",
        "application/gzip" => "gz",

        // 其他
        _ => "bin",
    }
}

/// 验证MIME类型与fileType是否匹配
pub fn validate_file_type(detected_mime: &str, file_type: &str) -> AppResult<()> {
    match file_type {
        "image" => {
            if !detected_mime.starts_with("image/") {
                return Err(AppError::UnsupportedFileType(
                    format!("Invalid file type. Expected image, got: {}", detected_mime)
                ));
            }
        },
        "video" => {
            if !detected_mime.starts_with("video/") {
                return Err(AppError::UnsupportedFileType(
                    format!("Invalid file type. Expected video, got: {}", detected_mime)
                ));
            }
        },
        "audio" => {
            if !detected_mime.starts_with("audio/") {
                return Err(AppError::UnsupportedFileType(
                    format!("Invalid file type. Expected audio, got: {}", detected_mime)
                ));
            }
        },
        "document" => {
            const ALLOWED_DOC_TYPES: &[&str] = &[
                "application/pdf",
                "application/msword",
                "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
                "application/vnd.ms-excel",
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "application/vnd.ms-powerpoint",
                "application/vnd.openxmlformats-officedocument.presentationml.presentation",
                "text/plain",
                "text/rtf",
                "text/csv",
            ];
            if !ALLOWED_DOC_TYPES.contains(&detected_mime) {
                return Err(AppError::UnsupportedFileType(
                    format!("Invalid document type: {}", detected_mime)
                ));
            }
        },
        "archive" => {
            const ALLOWED_ARCHIVE_TYPES: &[&str] = &[
                "application/zip",
                "application/x-rar-compressed",
                "application/x-7z-compressed",
                "application/x-tar",
                "application/gzip",
            ];
            if !ALLOWED_ARCHIVE_TYPES.contains(&detected_mime) {
                return Err(AppError::UnsupportedFileType(
                    format!("Invalid archive type: {}", detected_mime)
                ));
            }
        },
        _ => {
            // "other" 或未指定的类型，允许任何文件
        }
    }
    Ok(())
}

/// 计算文件的SHA-256哈希值
pub fn calculate_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// 从文件内容和文件名检测MIME类型
pub fn detect_mime_type(file_name: &str, data: &[u8]) -> String {
    // 1. 优先使用基于文件内容的检测
    if let Some(kind) = infer::get(data) {
        // 使用文件内容检测到的MIME类型
        return kind.mime_type().to_string();
    }

    // 2. 如果内容检测失败，使用基于文件名的检测
    mime_guess::from_path(file_name)
        .first_or_octet_stream()
        .to_string()
}

/// 获取最大文件大小限制（字节）
pub fn get_max_file_size() -> usize {
    std::env::var("MAX_FILE_SIZE")
        .and_then(|s| s.parse().map_err(|_| std::env::VarError::NotPresent))
        .unwrap_or(100 * 1024 * 1024) // 默认100MB
}

/// 根据MIME类型判断是否是图片
pub fn is_image(mime_type: &str) -> bool {
    mime_type.starts_with("image/")
}

/// 根据MIME类型判断是否是视频
pub fn is_video(mime_type: &str) -> bool {
    mime_type.starts_with("video/")
}

/// 根据MIME类型判断是否是音频
pub fn is_audio(mime_type: &str) -> bool {
    mime_type.starts_with("audio/")
}

/// 根据MIME类型判断是否是文档
pub fn is_document(mime_type: &str) -> bool {
    const DOC_TYPES: &[&str] = &[
        "application/pdf",
        "application/msword",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "application/vnd.ms-excel",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "application/vnd.ms-powerpoint",
        "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        "text/plain",
        "text/rtf",
        "text/csv",
    ];
    DOC_TYPES.contains(&mime_type)
}

/// 根据MIME类型判断是否是压缩文件
pub fn is_archive(mime_type: &str) -> bool {
    const ARCHIVE_TYPES: &[&str] = &[
        "application/zip",
        "application/x-rar-compressed",
        "application/x-7z-compressed",
        "application/x-tar",
        "application/gzip",
    ];
    ARCHIVE_TYPES.contains(&mime_type)
}

/// 根据MIME类型自动分类
pub fn get_file_type_from_mime(mime_type: &str) -> String {
    if is_image(mime_type) {
        "image".to_string()
    } else if is_video(mime_type) {
        "video".to_string()
    } else if is_audio(mime_type) {
        "audio".to_string()
    } else if is_document(mime_type) {
        "document".to_string()
    } else if is_archive(mime_type) {
        "archive".to_string()
    } else {
        "other".to_string()
    }
}

/// 格式化文件大小为人类可读格式
pub fn format_file_size(size: i64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as i64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

/// 异步从磁盘读取文件内容
pub async fn read_file_from_disk(path: &str) -> AppResult<Vec<u8>> {
    let base_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    let full_path = format!("{}/{}", base_dir, path);

    // 安全检查：确保路径不包含目录遍历攻击
    if path.contains("..") || path.starts_with('/') {
        return Err(AppError::BadRequest("Invalid file path".to_string()));
    }

    tokio::fs::read(&full_path).await
        .map_err(|e| AppError::FileStorageError(format!("Failed to read file: {}", e)))
}

/// 创建文件下载响应
pub fn create_download_response(
    file_content: Vec<u8>,
    file_name: &str,
    mime_type: &str,
) -> axum::response::Response {
    use axum::{
        http::{header, StatusCode},
    };

    // URL编码文件名
    let encoded_filename = utf8_percent_encode(file_name, NON_ALPHANUMERIC).to_string();
    let content_disposition = format!("attachment; filename=\"{}\"", encoded_filename);

    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime_type)
        .header(header::CONTENT_LENGTH, file_content.len())
        .header(header::CONTENT_DISPOSITION, content_disposition)
        .body(axum::body::Body::from(file_content))
        .unwrap()
}