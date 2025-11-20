use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::repository::GroupChatRepository;

#[async_trait]
impl GroupChatRepository for MySqlPool {
    
}