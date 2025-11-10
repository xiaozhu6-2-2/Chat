use async_trait::async_trait;
use bb8_redis::redis::pipe;
use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;

use crate::models::{entities::UserOnline, errors::{AppError, AppResult}, repository::OnlineRepository};

pub struct OnlineManager;

#[async_trait]
impl OnlineRepository for OnlineManager {
    // 在线状态上线
    async fn user_online(
        redis_pool : &Pool<RedisConnectionManager>,
        infomation : UserOnline,
        group_ids : &[String]
    ) -> AppResult<()> {
        // 获取Redis连接
        let mut conn = redis_pool.get().await.map_err(|e| AppError::RedisGetConnFailure(e.to_string()))?;

        // 全局在线状态key
        let global_key = "global:online:users".to_string();

        // 管道批处理
        let mut pipe = pipe();

        pipe.atomic()
            .sadd(&global_key, &infomation.account)
            .expire(&global_key, 300) // 300秒的过期时间
            .ignore();

        // 群聊在线状态
        for group_id in group_ids {
            let group_key = format!("group:online:{}", group_id);
            pipe.sadd(&group_key, &infomation.account).expire(&group_key, 300).ignore();
        }

        // 执行操作
        let _ : () = pipe.query_async(&mut *conn).await
            .map_err(|e| AppError::RedisOperationFailure(e.to_string()))?;

        Ok(())
    }

    // 在线状态下线
    async fn user_offline(
        redis_pool : &Pool<RedisConnectionManager>,
        account : String,
        group_ids : &[String]
    ) -> AppResult<()> {
        // 获取Redis连接
        let mut conn = redis_pool.get().await.map_err(|e| AppError::RedisGetConnFailure(e.to_string()))?;

        // 全局在线状态key
        let global_key = "global:online:users".to_string();

        // 管道批处理
        let mut pipe = pipe();

        pipe.atomic().srem(&global_key, &account).ignore();

        // 群聊在线状态
        for group_id in group_ids {
            let group_key = format!("group:online:{}", group_id);
            pipe.srem(&group_key, &account).ignore();
        }

        let _ : () = pipe.query_async(&mut *conn).await.map_err(|e| AppError::RedisOperationFailure(e.to_string()))?;

        Ok(())
    }

    // 心跳更新
    async fn update_heartbeat(
        redis_pool : &Pool<RedisConnectionManager>,
        account : String,
        group_ids : &[String]
    ) -> AppResult<()> {
        // 获取Redis连接
        let mut conn = redis_pool.get().await.map_err(|e| AppError::RedisGetConnFailure(e.to_string()))?;

        // 全局在线状态key
        let global_key = "global:online:users".to_string();

        // 管道批处理
        let mut pipe = pipe();

        // 更新全局在线状态
        pipe.expire(&global_key, 300).ignore();

        // 更新群聊在线状态
        for group_id in group_ids {
            let group_key = format!("group:online:{}", group_id);
            pipe.expire(&group_key, 300).ignore();
        }

        let _ : () = pipe.query_async(&mut *conn).await.map_err(|e| AppError::RedisOperationFailure(e.to_string()))?;
    
        Ok(())
    }
}