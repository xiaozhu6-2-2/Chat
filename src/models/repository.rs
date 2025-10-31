// src/models/repository.rs
use async_trait::async_trait;

use crate::models::{entities::User, errors::AppResult};

// 对用户表数据操作定义的接口
#[async_trait]
pub trait UserRepository: Send + Sync {
    // 根据账号查找用户
    async fn find_user_by_account(&self, account: &str) ->AppResult<User>;
}