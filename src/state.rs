// src/state.rs
// 库模块导入
use axum::extract::ws::Message;
use sqlx::MySqlPool;
use std::sync::Arc;
use tokio::sync::mpsc;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rand_core::OsRng;
use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;
use dashmap::DashMap;
// 模块分离导入
use crate::{models::{errors::{AppError, AppResult}, others::GroupBroadcastChannel}, utils::{connection_resources_manager::ConnectionResourcesManager, group_listener_manager::UserGroupTaskManager}};

#[derive(Clone)]
pub struct AppState {
    // 数据库的连接池
    pub db_pool: MySqlPool,
    // redis的连接池
    pub redis_pool: Pool<RedisConnectionManager>,
    // 密钥对
    pub session_key: (RsaPrivateKey, RsaPublicKey),
    // 对接WebSocket的写端的mpsc发送端池
    pub connection_pool: Arc<DashMap<String, mpsc::UnboundedSender<Message>>>,
    // 群聊广播频道池
    pub broadcast_pool: Arc<DashMap<String, GroupBroadcastChannel>>,
    // 群聊监听任务管理器
    pub group_task_manager: Arc<UserGroupTaskManager>,
    // 连接资源管理器
    pub connection_resources_manager: Arc<DashMap<String, ConnectionResourcesManager>>,
}

impl AppState {
    pub async fn new(db_pool: MySqlPool) -> AppResult<Self> {
        // 构建redis连接池
        let manager = RedisConnectionManager::new("redis://localhost:6379")
            .map_err(|e| AppError::StateGenerationFailure(e.to_string()))?;
        let pool = Pool::builder()
            .max_size(15) // 最大连接数
            .min_idle(Some(5)) // 最小空闲连接数
            .build(manager)
            .await
            .map_err(|e| AppError::StateGenerationFailure(e.to_string()))?;

        Ok(Self {
            db_pool,
            redis_pool: pool,
            session_key: generate_keys(2048),
            connection_pool: Arc::new(DashMap::new()), 
            broadcast_pool: Arc::new(DashMap::new()),
            group_task_manager: Arc::new(UserGroupTaskManager::new()),
            connection_resources_manager: Arc::new(DashMap::new())
        })
    }
}

fn generate_keys(bits : usize) -> (RsaPrivateKey, RsaPublicKey) {
    let private_key = RsaPrivateKey::new(&mut OsRng, bits).expect("密钥生成失败");
    let public_key = RsaPublicKey::from(&private_key);
    (private_key, public_key)
} 