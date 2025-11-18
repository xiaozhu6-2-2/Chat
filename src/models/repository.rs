// src/models/repository.rs
use async_trait::async_trait;
use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8::Pool;

use crate::models::{entities::{User, UserOnline}, errors::AppResult};

// 对用户表数据操作定义的接口
#[async_trait]
pub trait UserRepository: Send + Sync {
    // 根据账号查找用户
    async fn find_user_by_account(&self, account: &str) -> AppResult<User>;
    // 插入用户
    async fn insert_user(&self, user: User) -> AppResult<()>;
}

#[async_trait]
pub trait OnlineRepository: Send + Sync {
    // 在线状态上线
    async fn user_online(
        redis_pool : &Pool<RedisConnectionManager>,
        infomation : UserOnline,
        group_ids : &[String]
    ) -> AppResult<()>;

    // 在线状态下线
    async fn user_offline(
        redis_pool : &Pool<RedisConnectionManager>,
        account : String,
        group_ids : &[String]
    ) -> AppResult<()>;

    // 更新心跳
    async fn update_heartbeat(
        redis_pool : &Pool<RedisConnectionManager>,
        group_ids : &[String]
    ) -> AppResult<()>;
}