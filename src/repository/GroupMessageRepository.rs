use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::repository::GroupMessageRepository;

#[async_trait]
impl GroupMessageRepository for MySqlPool {
    
}