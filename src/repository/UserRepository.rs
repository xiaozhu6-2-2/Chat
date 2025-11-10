use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::{entities::User, errors::{AppError, AppResult}, repository::UserRepository};

#[async_trait]
impl UserRepository for MySqlPool {
    // 从Mysql数据库中按账号查询并返回User实体
    async fn find_user_by_account(&self, account: &str) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM user_info WHERE account = ?",
            account
        ).fetch_optional(self).await?;

        user.ok_or_else(|| {
            AppError::UserNotFound(account.to_string())
        })
    }
    // 插入User实体到数据库
    async fn insert_user(&self, user: User) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO user_info (account, password, username) VALUES (?, ?, ?)",
            user.account,
            user.password,
            user.username
        ).execute(&mut *tx).await?;

        // 插入结束
        tx.commit().await?;
        
        Ok(())
    }
}