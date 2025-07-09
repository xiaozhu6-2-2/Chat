// src/state.rs
use sqlx::MySqlPool;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: MySqlPool,
}

impl AppState {
    /// 创建带数据库连接池的应用状态
    pub fn new(db_pool: MySqlPool) -> Self {
        Self { db_pool }
    }
    
    /// 从环境变量初始化（高级用法）
    pub async fn from_env() -> sqlx::Result<Self> {
        let db_url = std::env::var("DATABASE_URL")
            .expect("DATABASE_URL must be set");
        let pool = MySqlPool::connect(&db_url).await?;
        Ok(Self::new(pool))
    }
}