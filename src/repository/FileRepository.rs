use sqlx::MySqlPool;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use tokio::fs;

use crate::models::{
    entities::{
        FileStorage, FileMetadata, FilePermission, FileAssociation,
        AccessTarget, AccessLevel, AssociationType, FileStatus
    },
    errors::{AppResult, AppError},
    repository::FileRepository
};

/// 已删除文件的占位符存储ID
const DELETED_FILES_STORAGE_ID: &str = "deleted_files_placeholder";

#[async_trait]
impl FileRepository for MySqlPool {
    // ==================== 文件存储管理 (file_storage) ====================

    /// 创建或获取文件存储记录（基于哈希去重）
    async fn create_or_get_file_storage(
        &self,
        file_hash: &str,
        file_path: &str,
        thumbnail_path: Option<String>,
        file_size: i64,
        mime_type: &str,
    ) -> AppResult<FileStorage> {
        // 1. 先查询是否已存在该哈希的文件
        let existing = sqlx::query_as!(
            FileStorage,
            "SELECT *
            FROM file_storage
            WHERE file_hash = ?",
            file_hash
        )
        .fetch_optional(self)
        .await?;

        if let Some(mut storage) = existing {
            // 2. 如果存在，增加引用计数
            sqlx::query!(
                "UPDATE file_storage SET reference_count = reference_count + 1 WHERE storage_id = ?",
                storage.storage_id
            )
            .execute(self)
            .await?;

            // 手动更新引用计数（因为查询到的是更新前的值）
            storage.reference_count = storage.reference_count.map(|count| count + 1);

            Ok(storage)
        } else {
            // 3. 如果不存在，使用雪花算法生成 storage_id
            let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
            let storage_id = snowflake.next_id()?.to_string();

            // 4. 插入新记录（created_at、reference_count、storage_location 由数据库默认值处理）
            sqlx::query!(
                "INSERT INTO file_storage (
                    storage_id, file_hash, file_path, thumbnail_path,
                    file_size, mime_type
                ) VALUES (?, ?, ?, ?, ?, ?)",
                storage_id,
                file_hash,
                file_path,
                thumbnail_path,
                file_size,
                mime_type
            )
            .execute(self)
            .await?;

            // 5. 查询并返回新创建的记录
            let new_storage = sqlx::query_as!(
                FileStorage,
                "SELECT *
                FROM file_storage
                WHERE storage_id = ?",
                storage_id
            )
            .fetch_one(self)
            .await?;

            Ok(new_storage)
        }
    }

    /// 根据storage_id获取文件存储信息
    async fn find_file_storage_by_id(&self, storage_id: &str) -> AppResult<Option<FileStorage>> {
        let storage = sqlx::query_as!(
            FileStorage,
            "SELECT *
            FROM file_storage
            WHERE storage_id = ?",
            storage_id
        )
        .fetch_optional(self)
        .await?;

        Ok(storage)
    }

    /// 根据文件哈希获取文件存储信息
    async fn find_file_storage_by_hash(&self, file_hash: &str) -> AppResult<Option<FileStorage>> {
        let storage = sqlx::query_as!(
            FileStorage,
            "
            SELECT *
            FROM file_storage
            WHERE file_hash = ?",
            file_hash
        )
        .fetch_optional(self)
        .await?;

        Ok(storage)
    }

    /// 增加文件引用计数
    async fn increment_reference_count(&self, storage_id: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE file_storage SET reference_count = reference_count + 1 WHERE storage_id = ?",
            storage_id
        )
        .execute(self)
        .await?;

        Ok(())
    }
  
    /// 获取无引用的文件（用于清理）
    async fn find_unused_files(&self) -> AppResult<Vec<FileStorage>> {
        let files = sqlx::query_as!(
            FileStorage,
            "
            SELECT *
            FROM file_storage
            WHERE reference_count <= 0
            "
        )
        .fetch_all(self)
        .await?;

        Ok(files)
    }

    /// 删除文件存储记录（安全删除，使用占位符替换）
    async fn delete_file_storage(&self, storage_id: &str) -> AppResult<()> {
        let mut tx = self.begin().await?;

        // 1. 检查storage记录是否存在
        let storage = sqlx::query_as!(
            FileStorage,
            "SELECT * FROM file_storage WHERE storage_id = ?",
            storage_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if storage.is_none() {
            // 文件不存在，直接返回成功
            tx.commit().await?;
            return Ok(());
        }

        let file_storage = storage.unwrap();

        // 2. 检查引用计数是否小于等于0
        if file_storage.reference_count.unwrap_or(0) > 0 {
            tx.commit().await?;
            return Err(crate::models::errors::AppError::BadRequest(
                format!("文件仍有引用，无法删除。当前引用计数: {}", file_storage.reference_count.unwrap_or(0))
            ));
        }

        // 3. 确保占位符记录存在
        let placeholder_exists = sqlx::query_as!(
            FileStorage,
            "SELECT * FROM file_storage WHERE storage_id = ?",
            DELETED_FILES_STORAGE_ID
        )
        .fetch_optional(&mut *tx)
        .await?;

        if placeholder_exists.is_none() {
            // 创建占位符记录
            sqlx::query!(
                r#"
                INSERT INTO file_storage (
                    storage_id, file_hash, file_path, file_size, mime_type,
                    created_at, reference_count, storage_location
                ) VALUES (?, 'deleted_placeholder', 'deleted_placeholder', 0, 'application/octet-stream', UTC_TIMESTAMP(), 0, 'placeholder')
                "#,
                DELETED_FILES_STORAGE_ID
            )
            .execute(&mut *tx)
            .await?;
        }

        // 4. 将所有指向要被删除的storage_id的元数据全都指向占位符记录
        let updated_count = sqlx::query!(
            "UPDATE file_metadata SET storage_id = ? WHERE storage_id = ?",
            DELETED_FILES_STORAGE_ID,
            storage_id
        )
        .execute(&mut *tx)
        .await?
        .rows_affected();

        // 5. 现在可以安全删除原存储记录
        sqlx::query!(
            "DELETE FROM file_storage WHERE storage_id = ?",
            storage_id
        )
        .execute(&mut *tx)
        .await?;

        // 提交事务
        tx.commit().await?;

        // 可选：记录日志
        if updated_count > 0 {
            // 这里可以添加日志记录
            eprintln!("安全删除文件存储记录: {}, 已将 {} 个元数据记录重定向到占位符", storage_id, updated_count);
        }

        Ok(())
    }

    // ==================== 文件元数据管理 (file_metadata) ====================

    /// 创建文件元数据
    async fn create_file_metadata(
        &self,
        file_id: &str,
        storage_id: &str,
        owner_uid: &str,
        original_name: &str,
        display_name: &str,
        file_type: &str,
    ) -> AppResult<()> {
        // 1. 首先验证 storage_id 是否存在
        let storage_exists = sqlx::query!(
            "SELECT storage_id FROM file_storage WHERE storage_id = ?",
            storage_id
        )
        .fetch_optional(self)
        .await?;

        if storage_exists.is_none() {
            return Err(crate::models::errors::AppError::NotFound(
                format!("Storage record not found: {}", storage_id)
            ));
        }

        // 2. 创建文件元数据（upload_time、last_access_time、download_count、file_status 使用数据库默认值）
        sqlx::query!(
            "
            INSERT INTO file_metadata (
                file_id, storage_id, owner_uid, original_name,
                display_name, file_type
            ) VALUES (?, ?, ?, ?, ?, ?)
            ",
            file_id,
            storage_id,
            owner_uid,
            original_name,
            display_name,
            file_type
        )
        .execute(self)
        .await?;

        Ok(())
    }

    /// 根据file_id获取文件元数据
    async fn find_file_metadata_by_id(&self, file_id: &str) -> AppResult<Option<FileMetadata>> {
        let metadata = sqlx::query_as!(
            FileMetadata,
            "
            SELECT
                file_id, storage_id, owner_uid, original_name, display_name,
                file_type, upload_time, last_access_time, download_count,
                file_status as `file_status: FileStatus`
            FROM file_metadata
            WHERE file_id = ? AND file_status = 'active'
            ",
            file_id
        )
        .fetch_optional(self)
        .await?;

        Ok(metadata)
    }

    /// 根据所有者获取文件列表
    async fn find_files_by_owner(&self, owner_uid: &str, limit: Option<u32>, offset: Option<u32>) -> AppResult<Vec<FileMetadata>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let files = sqlx::query_as!(
            FileMetadata,
            "
            SELECT
                file_id, storage_id, owner_uid, original_name,
                display_name, file_type, upload_time, last_access_time,
                download_count, file_status as `file_status: FileStatus`
            FROM file_metadata
            WHERE owner_uid = ? AND file_status = 'active'
            ORDER BY upload_time DESC
            LIMIT ? OFFSET ?
            ",
            owner_uid,
            limit,
            offset
        )
        .fetch_all(self)
        .await?;

        Ok(files)
    }

    /// 更新文件访问时间
    async fn update_last_access_time(&self, file_id: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE file_metadata SET last_access_time = UTC_TIMESTAMP() WHERE file_id = ?",
            file_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    /// 增加下载次数
    async fn increment_download_count(&self, file_id: &str) -> AppResult<()> {
        sqlx::query!(
            "UPDATE file_metadata SET download_count = download_count + 1 WHERE file_id = ?",
            file_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    /// 软删除文件（集成减少引用计数和删除storage记录）
    async fn soft_delete_file(&self, file_id: &str) -> AppResult<()> {
        let mut tx = self.begin().await?;

        // 1. 获取文件元数据信息
        let metadata = sqlx::query_as!(
            FileMetadata,
            "
            SELECT
                file_id, storage_id, owner_uid, original_name,
                display_name, file_type, upload_time, last_access_time,
                download_count,
                file_status as `file_status: FileStatus`
            FROM file_metadata
            WHERE file_id = ?
            ",
            file_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if metadata.is_none() {
            // 文件不存在，返回错误
            return Err(crate::models::errors::AppError::NotFound(
                format!("File not found: {}", file_id)
            ));
        }

        let meta = metadata.unwrap();

        // 2. 标记文件元数据为已删除
        sqlx::query!(
            "UPDATE file_metadata SET file_status = 'deleted' WHERE file_id = ?",
            file_id
        )
        .execute(&mut *tx)
        .await?;

        // 3. 获取文件存储信息并处理引用计数
        let storage = sqlx::query_as!(
            FileStorage,
            "SELECT * FROM file_storage WHERE storage_id = ?",
            meta.storage_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if let Some(file_storage) = storage {
            let current_count = file_storage.reference_count.unwrap_or(0);

            if current_count <= 1 {
                // 4. 如果引用计数为0或1，准备删除storage记录
                // 4.1 首先标记所有使用该storage的元数据为deleted
                sqlx::query!(
                    "UPDATE file_metadata SET file_status = 'deleted' WHERE storage_id = ? AND file_status = 'active'",
                    meta.storage_id
                )
                .execute(&mut *tx)
                .await?;

                // 4.2 确保占位符记录存在
                let placeholder_exists = sqlx::query_as!(
                    FileStorage,
                    "SELECT * FROM file_storage WHERE storage_id = ?",
                    DELETED_FILES_STORAGE_ID
                )
                .fetch_optional(&mut *tx)
                .await?;

                if placeholder_exists.is_none() {
                    // 创建占位符记录
                    sqlx::query!(
                        r#"
                        INSERT INTO file_storage (
                            storage_id, file_hash, file_path, file_size, mime_type,
                            created_at, reference_count, storage_location
                        ) VALUES (?, 'deleted_placeholder', 'deleted_placeholder', 0, 'application/octet-stream', UTC_TIMESTAMP(), 0, 'placeholder')
                        "#,
                        DELETED_FILES_STORAGE_ID
                    )
                    .execute(&mut *tx)
                    .await?;
                }

                // 4.3 将所有指向该storage的元数据重定向到占位符
                let updated_count = sqlx::query!(
                    "UPDATE file_metadata SET storage_id = ? WHERE storage_id = ?",
                    DELETED_FILES_STORAGE_ID,
                    meta.storage_id
                )
                .execute(&mut *tx)
                .await?
                .rows_affected();

                // 4.4.1 获取文件路径并删除物理文件
                let base_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
                let full_path = format!("{}/{}", base_dir, file_storage.file_path);

                // 删除物理文件
                if let Err(e) = fs::remove_file(&full_path).await {
                    eprintln!("警告：无法删除物理文件 {}: {}", full_path, e);
                    // 不中断事务，但记录警告
                } else {
                    eprintln!("成功删除物理文件: {}", full_path);
                }

                // 4.4.2 现在可以安全删除原存储记录
                sqlx::query!(
                    "DELETE FROM file_storage WHERE storage_id = ?",
                    meta.storage_id
                )
                .execute(&mut *tx)
                .await?;

                // 可选：记录日志
                if updated_count > 0 {
                    eprintln!("安全删除文件存储记录: {}, 已将 {} 个元数据记录重定向到占位符", meta.storage_id, updated_count);
                }
            } else {
                // 5. 否则正常减少引用计数
                sqlx::query!(
                    "UPDATE file_storage SET reference_count = reference_count - 1 WHERE storage_id = ?",
                    meta.storage_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        // 6. 提交事务
        tx.commit().await?;
        Ok(())
    }

    // ==================== 文件权限管理 (file_permission) ====================

    /// 授予文件权限（幂等实现）
    async fn grant_file_permission(
        &self,
        file_id: &str,
        access_type: AccessTarget,
        target_id: Option<String>,
        permission_level: AccessLevel,
        granted_by: &str,
        expires_at: Option<DateTime<Utc>>,
    ) -> AppResult<()> {
        // 1. 验证文件是否存在
        let file_exists = self.find_file_metadata_by_id(file_id).await?;
        if file_exists.is_none() {
            return Err(AppError::NotFound(format!("File not found: {}", file_id)));
        }

        // 2. 根据 access_type 验证 target_id
        match access_type {
            AccessTarget::User => {
                // User 类型必须有 target_id 且该用户必须存在
                let uid = target_id.as_ref().ok_or_else(|| AppError::BadRequest("User access type requires target_id".to_string()))?;
                use crate::models::repository::UserRepository;
                let _user = self.find_user_by_uid(uid).await.map_err(|_| AppError::NotFound(format!("User not found: {}", uid)))?;
            },
            AccessTarget::Group => {
                // Group 类型必须有 target_id 且该群组必须存在
                let gid = target_id.as_ref().ok_or_else(|| AppError::BadRequest("Group access type requires target_id".to_string()))?;
                use crate::models::repository::GroupChatRepository;
                let _group = self.find_group_by_gid(gid).await?.ok_or_else(|| AppError::NotFound(format!("Group not found: {}", gid)))?;
            },
            AccessTarget::Friend => {
                // Friend 类型应该没有 target_id
                if target_id.is_some() {
                    return Err(AppError::BadRequest("Friend access type should not have target_id".to_string()));
                }
            },
            AccessTarget::Public => {
                // Public 类型应该没有 target_id
                if target_id.is_some() {
                    return Err(AppError::BadRequest("Public access type should not have target_id".to_string()));
                }
            },
        }

        // 3. 先检查权限是否已存在（幂等性检查）
        if check_permission_exists(self, file_id, access_type.clone(), target_id.clone()).await? {
            // 权限已存在，直接返回成功
            return Ok(());
        }

        // 4. 开始事务，处理可能的竞态条件
        let mut tx = self.begin().await?;

        // 5. 在事务内再次检查权限（双重检查模式）
        let permission_exists = sqlx::query!(
            r#"
            SELECT permission_id
            FROM file_permission
            WHERE file_id = ?
            AND access_type = ?
            AND (target_id = ? OR (target_id IS NULL AND ? IS NULL))
            "#,
            file_id,
            match access_type {
                AccessTarget::User => "user",
                AccessTarget::Friend => "friend",
                AccessTarget::Group => "group",
                AccessTarget::Public => "public",
            },
            target_id,
            target_id
        )
        .fetch_optional(&mut *tx)
        .await?;

        if permission_exists.is_some() {
            // 事务内发现权限已存在，提交事务并返回
            tx.commit().await?;
            return Ok(());
        }

        // 6. 权限不存在，插入新记录
        let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
        let permission_id = snowflake.next_id()?.to_string();

        sqlx::query!(
            "
            INSERT INTO file_permission (
                permission_id, file_id, access_type, target_id,
                permission_level, granted_by, expires_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?)
            ",
            permission_id,
            file_id,
            match access_type {
                AccessTarget::User => "user",
                AccessTarget::Friend => "friend",
                AccessTarget::Group => "group",
                AccessTarget::Public => "public",
            },
            target_id,
            match permission_level {
                AccessLevel::View => "view",
                AccessLevel::Download => "download",
                AccessLevel::Share => "share",
                AccessLevel::Manage => "manage",
            },
            granted_by,
            expires_at
        )
        .execute(&mut *tx)
        .await?;

        // 7. 提交事务
        tx.commit().await?;

        Ok(())
    }

    /// 验证文件访问权限（核心方法）
    async fn verify_file_permission(
        &self,
        file_id: &str,
        user_uid: &str,
        required_level: AccessLevel,
    ) -> AppResult<bool> {
        // 1. 首先检查文件是否存在并获取文件信息
        let metadata = self.find_file_metadata_by_id(file_id).await?;
        let meta = metadata.ok_or_else(|| AppError::NotFound(format!("文件 {} 不存在", file_id)))?;

        // 2. 检查是否是文件所有者（所有者拥有所有权限）
        if meta.owner_uid == user_uid {
            return Ok(true);
        }

        // 3. 检查权限级别是否满足要求的辅助函数
        let check_permission_level = |perm_level: AccessLevel, required: AccessLevel| -> bool {
            let current = match perm_level {
                AccessLevel::View => 1,
                AccessLevel::Download => 2,
                AccessLevel::Share => 3,
                AccessLevel::Manage => 4,
            };
            let required = match required {
                AccessLevel::View => 1,
                AccessLevel::Download => 2,
                AccessLevel::Share => 3,
                AccessLevel::Manage => 4,
            };
            current >= required
        };

        // 4. 首先检查 public 权限（如果有，则所有人都有权限）
        if let Some(public_perm) = sqlx::query_as!(
            FilePermission,
            r#"
            SELECT
                permission_id, file_id, access_type as `access_type: AccessTarget`, target_id,
                permission_level as `permission_level: AccessLevel`, granted_by,
                granted_at, expires_at
            FROM file_permission
            WHERE file_id = ?
            AND access_type = 'public'
            AND target_id IS NULL
            AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP())
            ORDER BY
                CASE permission_level
                    WHEN 'manage' THEN 4
                    WHEN 'share' THEN 3
                    WHEN 'download' THEN 2
                    WHEN 'view' THEN 1
                    ELSE 0
                END DESC
            LIMIT 1
            "#,
            file_id
        )
        .fetch_optional(self)
        .await?
        {
            return Ok(check_permission_level(public_perm.permission_level, required_level));
        }

        // 5. 检查 user 权限（直接授权给特定用户）
        if let Some(user_perm) = sqlx::query_as!(
            FilePermission,
            r#"
            SELECT
                permission_id, file_id, access_type as `access_type: AccessTarget`, target_id,
                permission_level as `permission_level: AccessLevel`, granted_by,
                granted_at, expires_at
            FROM file_permission
            WHERE file_id = ?
            AND access_type = 'user'
            AND target_id = ?
            AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP())
            ORDER BY
                CASE permission_level
                    WHEN 'manage' THEN 4
                    WHEN 'share' THEN 3
                    WHEN 'download' THEN 2
                    WHEN 'view' THEN 1
                    ELSE 0
                END DESC
            LIMIT 1
            "#,
            file_id,
            user_uid
        )
        .fetch_optional(self)
        .await?
        {
            return Ok(check_permission_level(user_perm.permission_level, required_level));
        }

        // 6. 检查 group 权限（用户所属的群组权限）
        // 首先获取用户加入的所有群组
        let user_groups = sqlx::query!(
            "
            SELECT gm.gid
            FROM group_member gm
            WHERE gm.uid = ?
            ",
            user_uid
        )
        .fetch_all(self)
        .await?;

        if !user_groups.is_empty() {
            let group_ids: Vec<String> = user_groups.iter().map(|g| g.gid.clone()).collect();
            let placeholders = group_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");

            let query = format!(
                r#"
                SELECT
                    permission_id, file_id, access_type, target_id,
                    permission_level, granted_by,
                    granted_at, expires_at
                FROM file_permission
                WHERE file_id = ?
                AND access_type = 'group'
                AND target_id IN ({})
                AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP())
                ORDER BY
                    CASE permission_level
                        WHEN 'manage' THEN 4
                        WHEN 'share' THEN 3
                        WHEN 'download' THEN 2
                        WHEN 'view' THEN 1
                        ELSE 0
                    END DESC
                LIMIT 1
                "#,
                placeholders
            );

            let mut query_builder = sqlx::query(&query).bind(file_id);
            for group_id in &group_ids {
                query_builder = query_builder.bind(group_id);
            }

            if let Some(row) = query_builder.fetch_optional(self).await? {
                // 手动获取字段并转换为枚举
                let access_type_str: String = row.try_get("access_type")?;
                let permission_level_str: String = row.try_get("permission_level")?;

                let group_perm = FilePermission {
                    permission_id: row.try_get("permission_id")?,
                    file_id: row.try_get("file_id")?,
                    access_type: AccessTarget::from_enum_string(&access_type_str)
                        .ok_or_else(|| AppError::InternalError(format!("Invalid access_type: {}", access_type_str)))?,
                    target_id: row.try_get("target_id")?,
                    permission_level: AccessLevel::from_enum_string(&permission_level_str)
                        .ok_or_else(|| AppError::InternalError(format!("Invalid permission_level: {}", permission_level_str)))?,
                    granted_by: row.try_get("granted_by")?,
                    granted_at: row.try_get("granted_at")?,
                    expires_at: row.try_get("expires_at")?,
                };
                return Ok(check_permission_level(group_perm.permission_level, required_level));
            }
        }

        // 7. 检查 friend 权限（用户是某个授权者的好友）
        // 首先获取所有 friend 类型的权限记录
        let friend_perms = sqlx::query_as!(
            FilePermission,
            r#"
            SELECT
                permission_id, file_id, access_type as `access_type: AccessTarget`, target_id,
                permission_level as `permission_level: AccessLevel`, granted_by,
                granted_at, expires_at
            FROM file_permission
            WHERE file_id = ?
            AND access_type = 'friend'
            AND target_id IS NULL
            AND (expires_at IS NULL OR expires_at > UTC_TIMESTAMP())
            ORDER BY
                CASE permission_level
                    WHEN 'manage' THEN 4
                    WHEN 'share' THEN 3
                    WHEN 'download' THEN 2
                    WHEN 'view' THEN 1
                    ELSE 0
                END DESC
            "#,
            file_id
        )
        .fetch_all(self)
        .await?;

        // 对每个 friend 权限记录，检查用户是否是该授权者的好友
        for friend_perm in friend_perms {
            let is_friend = sqlx::query!(
                r#"
                SELECT 1 as `exists`
                FROM friends
                WHERE ((uid = ? AND to_uid = ?) OR (uid = ? AND to_uid = ?))
                AND is_blacklist = 0 AND to_is_blacklist = 0
                LIMIT 1
                "#,
                user_uid, friend_perm.granted_by, friend_perm.granted_by, user_uid
            )
            .fetch_optional(self)
            .await?
            .is_some();

            if is_friend {
                // 如果是好友，检查权限级别是否满足要求
                if check_permission_level(friend_perm.permission_level, required_level) {
                    return Ok(true);
                }
            }
        }

        // 8. 没有找到任何匹配的权限
        Ok(false)
    }

    /// 撤销文件权限（按访问类型和目标ID删除）
    async fn revoke_file_permission(
        &self,
        file_id: &str,
        access_type: AccessTarget,
        target_id: &str,
    ) -> AppResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM file_permission
            WHERE file_id = ? AND access_type = ? AND target_id = ?
            "#,
            file_id,
            match access_type {
                AccessTarget::User => "user",
                AccessTarget::Friend => "friend",
                AccessTarget::Group => "group",
                AccessTarget::Public => "public",
            },
            target_id
        )
        .execute(self)
        .await?;

        Ok(result.rows_affected())
    }

    // ==================== 文件关联管理 (file_association) ====================

    /// 创建文件关联
    async fn create_file_association(
        &self,
        file_id: &str,
        association_type: AssociationType,
        associated_id: &str,
        creator_uid: &str,
    ) -> AppResult<()> {
        // 1. 验证 file_id 是否存在
        let file_exists = sqlx::query!(
            "SELECT file_id FROM file_metadata WHERE file_id = ?",
            file_id
        )
        .fetch_optional(self)
        .await?
        .is_some();

        if !file_exists {
            return Err(AppError::NotFound(format!("File {} not found", file_id)));
        }

        // 2. 根据 association_type 验证 associated_id
        match association_type {
            AssociationType::PrivateMessage | AssociationType::GroupMessage | AssociationType::PostAttachment => {
                // 验证消息是否存在
                let message_exists = match association_type {
                    AssociationType::PrivateMessage => {
                        sqlx::query!("SELECT msg_id FROM private_message WHERE msg_id = ?", associated_id)
                            .fetch_optional(self)
                            .await?
                            .is_some()
                    }
                    AssociationType::GroupMessage => {
                        sqlx::query!("SELECT msg_id FROM group_message WHERE msg_id = ?", associated_id)
                            .fetch_optional(self)
                            .await?
                            .is_some()
                    }
                    AssociationType::PostAttachment => {
                        // Post attachments might be stored in a different table
                        // For now, we'll skip validation for post attachments as the table structure is unclear
                        true
                    }
                    _ => unreachable!(),
                };

                if !message_exists {
                    let msg_type = match association_type {
                        AssociationType::PrivateMessage => "private_message",
                        AssociationType::GroupMessage => "group_message",
                        AssociationType::PostAttachment => "post",
                        _ => unreachable!(),
                    };
                    return Err(AppError::NotFound(format!("Message {} not found in {}", associated_id, msg_type)));
                }
            }
            AssociationType::UserAvatar => {
                // 验证用户是否存在
                let user_exists = sqlx::query!(
                    "SELECT uid FROM user WHERE uid = ?",
                    associated_id
                )
                .fetch_optional(self)
                .await?
                .is_some();

                if !user_exists {
                    return Err(AppError::NotFound(format!("User {} not found", associated_id)));
                }
            }
            AssociationType::GroupAvatar => {
                // 验证群组是否存在
                let group_exists = sqlx::query!(
                    "SELECT gid FROM group_chat WHERE gid = ?",
                    associated_id
                )
                .fetch_optional(self)
                .await?
                .is_some();

                if !group_exists {
                    return Err(AppError::NotFound(format!("Group {} not found", associated_id)));
                }
            }
        }

        // 3. 创建关联记录
        let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
        let association_id = snowflake.next_id()?.to_string();

        sqlx::query!(
            r#"
            INSERT INTO file_association (
                association_id, file_id, association_type,
                associated_id, creator_uid
            ) VALUES (?, ?, ?, ?, ?)
            "#,
            association_id,
            file_id,
            match association_type {
                AssociationType::PrivateMessage => "private_message",
                AssociationType::GroupMessage => "group_message",
                AssociationType::UserAvatar => "user_avatar",
                AssociationType::GroupAvatar => "group_avatar",
                AssociationType::PostAttachment => "post_attachment",
            },
            associated_id,
            creator_uid
        )
        .execute(self)
        .await?;

        Ok(())
    }

    /// 根据关联查询文件
    async fn find_files_by_association(
        &self,
        association_type: AssociationType,
        associated_id: &str,
    ) -> AppResult<Vec<FileAssociation>> {
        let associations = sqlx::query_as!(
            FileAssociation,
            r#"
            SELECT
                association_id, file_id, association_type as `association_type: AssociationType`,
                associated_id, creator_uid, created_at
            FROM file_association
            WHERE association_type = ? AND associated_id = ?
            ORDER BY created_at DESC
            "#,
            match association_type {
                AssociationType::PrivateMessage => "private_message",
                AssociationType::GroupMessage => "group_message",
                AssociationType::UserAvatar => "user_avatar",
                AssociationType::GroupAvatar => "group_avatar",
                AssociationType::PostAttachment => "post_attachment",
            },
            associated_id
        )
        .fetch_all(self)
        .await?;

        Ok(associations)
    }

    /// 获取文件的所有关联
    async fn find_file_associations(&self, file_id: &str) -> AppResult<Vec<FileAssociation>> {
        let associations = sqlx::query_as!(
            FileAssociation,
            r#"
            SELECT
                association_id, file_id, association_type as `association_type: AssociationType`,
                associated_id, creator_uid, created_at
            FROM file_association
            WHERE file_id = ?
            ORDER BY created_at DESC
            "#,
            file_id
        )
        .fetch_all(self)
        .await?;

        Ok(associations)
    }

    /// 删除文件关联
    async fn delete_file_association(&self, association_id: &str) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM file_association WHERE association_id = ?",
            association_id
        )
        .execute(self)
        .await?;

        Ok(())
    }

    /// 批量删除关联（如删除消息时）
    async fn batch_delete_associations_by_target(
        &self,
        association_type: AssociationType,
        associated_id: &str,
    ) -> AppResult<u64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM file_association
            WHERE association_type = ? AND associated_id = ?
            "#,
            match association_type {
                AssociationType::PrivateMessage => "private_message",
                AssociationType::GroupMessage => "group_message",
                AssociationType::UserAvatar => "user_avatar",
                AssociationType::GroupAvatar => "group_avatar",
                AssociationType::PostAttachment => "post_attachment",
            },
            associated_id
        )
        .execute(self)
        .await?;

        Ok(result.rows_affected())
    }
}

// ==================== 辅助函数 ====================

/// 检查指定权限是否已存在
///
/// # 参数
/// * `pool` - MySQL连接池
/// * `file_id` - 文件ID
/// * `access_type` - 访问类型（User, Friend, Group, Public）
/// * `target_id` - 目标ID（对于Friend和Public类型应为None）
///
/// # 返回值
/// 返回 `AppResult<bool>`，如果权限存在返回true，否则返回false
pub async fn check_permission_exists(
    pool: &MySqlPool,
    file_id: &str,
    access_type: AccessTarget,
    target_id: Option<String>,
) -> AppResult<bool> {
    let permission = sqlx::query!(
        r#"
        SELECT permission_id
        FROM file_permission
        WHERE file_id = ?
        AND access_type = ?
        AND (target_id = ? OR (target_id IS NULL AND ? IS NULL))
        "#,
        file_id,
        match access_type {
            AccessTarget::User => "user",
            AccessTarget::Friend => "friend",
            AccessTarget::Group => "group",
            AccessTarget::Public => "public",
        },
        target_id,
        target_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(permission.is_some())
}

