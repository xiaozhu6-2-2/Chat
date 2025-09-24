// src/state.rs
// 库模块导入
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rand_core::OsRng;
use bb8_redis::bb8::Pool;
use bb8_redis::RedisConnectionManager;
// 模块分离导入
use crate::models::others::WsMessage;
use crate::models::entities::PrivateMessage;
use crate::models::errors::{AppError, AppResult};

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    pub redis_pool: Pool<RedisConnectionManager>,
    pub chat_rooms: Arc<Mutex<HashMap<u32, broadcast::Sender<WsMessage>>>>,
    pub online_users: Arc<Mutex<HashMap<u32, HashSet<String>>>>,
    pub private_sessions: Arc<Mutex<HashMap<u64, broadcast::Sender<PrivateMessage>>>>,
    pub session_key : (RsaPrivateKey, RsaPublicKey)
}

impl AppState {
    pub async fn new(db_pool: MySqlPool) -> AppResult<Self> {
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
            chat_rooms: Arc::new(Mutex::new(HashMap::new())),
            online_users : Arc::new(Mutex::new(HashMap::new())), 
            private_sessions: Arc::new(Mutex::new(HashMap::new())),
            session_key : generate_keys(2048)
        })
    }
}

fn generate_keys(bits : usize) -> (RsaPrivateKey, RsaPublicKey) {
    let private_key = RsaPrivateKey::new(&mut OsRng, bits).expect("密钥生成失败");
    let public_key = RsaPublicKey::from(&private_key);
    (private_key, public_key)
} 