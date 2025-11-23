// src/handlers/others.rs
/*
    这个模块是用来处理一些小请求
*/
// 库模块导入
use sqlx::MySqlPool;

use crate::models::errors::{AppError, AppResult};

// 分离模块导入


// 查询用户名
pub async fn get_username(db_pool: &MySqlPool, account: &str) -> AppResult<Option<String>> {
    let result = sqlx::query_scalar!(
            "SELECT username FROM user WHERE account = ?",
            account
        )
        .fetch_optional(db_pool)
        .await?;

    Ok(result)
}

