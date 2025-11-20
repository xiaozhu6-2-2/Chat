use sqlx::MySqlPool;
use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::models::{entities::User, errors::{AppError, AppResult}, repository::UserRepository};

#[async_trait]
impl UserRepository for MySqlPool {
    // 根据uid查找用户
    async fn find_user_by_uid(&self, uid: &str) -> AppResult<User> {}
    // 按账号查找用户
    async fn find_user_by_account(&self, account: &str) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT * FROM user WHERE account = ?",
            account
        ).fetch_optional(self).await?;

        user.ok_or_else(|| {
            AppError::UserNotFound(account.to_string())
        })
    }
    // 根据地区查找用户
    async fn find_user_by_region(&self, region: &str) -> AppResult<Vec<User>> {}
    // 根据用户名查找用户
    async fn find_user_by_username(&self, username: &str) -> AppResult<Vec<User>> {}
    // 根据创建时间查找用户
    async fn find_user_by_create_time_range(&self, start:NaiveDateTime, end:NaiveDateTime) -> AppResult<Vec<User>> {}

    // 插入User实体到数据库
    async fn insert_user(&self, user: User) -> AppResult<()> {
        // 生成uid
        
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO user (uid, account, password, username) VALUES (?, ?, ?, ?)",
            user.uid,
            user.account,
            user.password,
            user.username
        ).execute(&mut *tx).await?;

        // 插入结束
        tx.commit().await?;
        
        Ok(())
    }

    // 保存用户更改
    async fn save_user(&self, user: User) -> AppResult<()> {}

    // 删除用户
    async fn delete_user(&self, uid: &str) -> AppResult<()> {}

    // 根据账号判断用户是否存在
    async fn exists_by_account(&self, account: &str) -> AppResult<bool> {}
}