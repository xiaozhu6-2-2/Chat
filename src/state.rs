// src/state.rs
// 库模块导入
use sqlx::MySqlPool;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};
use rsa::{RsaPrivateKey, RsaPublicKey};
use rand_core::OsRng;
// 模块分离导入
use crate::models::WsMessage;
use crate::models::PrivateMessage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    pub chat_rooms: Arc<Mutex<HashMap<u32, broadcast::Sender<WsMessage>>>>,
    pub online_users: Arc<Mutex<HashMap<u32, HashSet<String>>>>,
    pub private_sessions: Arc<Mutex<HashMap<u64, broadcast::Sender<PrivateMessage>>>>,
    pub session_key : (RsaPrivateKey, RsaPublicKey)
}

impl AppState {
    pub fn new(db_pool: MySqlPool) -> Self {
        Self {
            db_pool,
            chat_rooms: Arc::new(Mutex::new(HashMap::new())),
            online_users : Arc::new(Mutex::new(HashMap::new())), 
            private_sessions: Arc::new(Mutex::new(HashMap::new())),
            session_key : generate_keys(2048)
        }
    }
}

fn generate_keys(bits : usize) -> (RsaPrivateKey, RsaPublicKey) {
    let private_key = RsaPrivateKey::new(&mut OsRng, bits).expect("密钥生成失败");
    let public_key = RsaPublicKey::from(&private_key);
    (private_key, public_key)
} 