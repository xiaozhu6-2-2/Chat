use sqlx::MySqlPool;
use async_trait::async_trait;
use chrono::NaiveDateTime;

use crate::models::{entities::User, errors::{AppError, AppResult}, repository::UserRepository};

#[async_trait]
impl UserRepository for MySqlPool {
    //---------------------------CRUD-------------------------------------
    // 根据uid查找用户
    async fn find_user_by_uid(&self, uid: &str) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT
                uid, account, password, username, gender, region,
                email, create_time, avatar
            FROM user WHERE uid = ?",
            uid
        ).fetch_optional(self).await?;

        user.ok_or_else(|| {
            AppError::UserNotFound(uid.to_string())
        })
    }

    // 按账号查找用户
    async fn find_user_by_account(&self, account: &str) -> AppResult<User> {
        let user = sqlx::query_as!(
            User,
            "SELECT
                uid, account, password, username, gender, region,
                email, create_time, avatar
            FROM user WHERE account = ?",
            account
        ).fetch_optional(self).await?;

        user.ok_or_else(|| {
            AppError::UserNotFound(account.to_string())
        })
    }

    // 根据地区查找用户
    async fn find_user_by_region(&self, region: &str) -> AppResult<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "SELECT
                uid, account, password, username, gender, region,
                email, create_time, avatar
            FROM user WHERE region = ?",
            region
        ).fetch_all(self).await?;

        Ok(users)
    }

    // 根据用户名查找用户（模糊匹配）
    async fn find_user_by_username(&self, username: &str) -> AppResult<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "SELECT
                uid, account, password, username, gender, region,
                email, create_time, avatar
            FROM user WHERE username LIKE ?",
            format!("%{}%", username)
        ).fetch_all(self).await?;

        Ok(users)
    }

    // 根据创建时间查找用户
    async fn find_user_by_create_time_range(&self, start: NaiveDateTime, end: NaiveDateTime) -> AppResult<Vec<User>> {
        let users = sqlx::query_as!(
            User,
            "SELECT
                uid, account, password, username, gender, region,
                email, create_time, avatar
            FROM user WHERE create_time BETWEEN ? AND ?
            ORDER BY create_time DESC",
            start,
            end
        ).fetch_all(self).await?;

        Ok(users)
    }

    // 插入User实体到数据库
    async fn insert_user(&self, user: User) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO user (uid, account, password, username, gender, region, email, create_time, avatar)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            user.uid,
            user.account,
            user.password,
            user.username,
            user.gender,
            user.region,
            user.email,
            user.create_time,
            user.avatar
        ).execute(&mut *tx).await?;

        // 插入结束
        tx.commit().await?;

        Ok(())
    }

    // 保存用户更改（使用 ON DUPLICATE KEY UPDATE 实现更新或插入）
    async fn save_user(&self, user: User) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO user (uid, account, password, username, gender, region, email, create_time, avatar)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
                account = VALUES(account),
                password = VALUES(password),
                username = VALUES(username),
                gender = VALUES(gender),
                region = VALUES(region),
                email = VALUES(email),
                avatar = VALUES(avatar)",
            user.uid,
            user.account,
            user.password,
            user.username,
            user.gender,
            user.region,
            user.email,
            user.create_time,
            user.avatar
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    // 删除用户
    async fn delete_user(&self, uid: &str) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "DELETE FROM user WHERE uid = ?",
            uid
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    //-----------------------------统计-------------------------------------
    // 根据账号判断用户是否存在
    async fn exists_by_account(&self, account: &str) -> AppResult<bool> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count FROM user WHERE account = ?",
            account
        ).fetch_one(self).await?;

        Ok(count.count > 0)
    }
}