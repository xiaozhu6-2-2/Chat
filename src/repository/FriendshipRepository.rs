use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::repository::FriendshipRepository;

#[async_trait]
impl FriendshipRepository for MySqlPool {
    
}