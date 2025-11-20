use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::repository::PrivateChatRepository;

#[async_trait]
impl PrivateChatRepository for MySqlPool {
    
}