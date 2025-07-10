// src/state.rs
// 库模块导入
use sqlx::MySqlPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

// 模块分离导入
use crate::models::WsMessage;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
    pub chat_rooms: Arc<Mutex<HashMap<u32, broadcast::Sender<WsMessage>>>>,
}

impl AppState {
    pub fn new(db_pool: MySqlPool) -> Self {
        Self {
            db_pool,
            chat_rooms: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}