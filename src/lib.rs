pub mod handlers;
pub mod models;
pub mod routes;
pub mod state;
pub mod middleware;
pub mod repository;
pub mod utils;

pub use state::AppState;

use tokio::net::TcpListener;
use sqlx::MySqlPool;

use crate::models::errors::{AppError, AppResult};

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub database_url: String,
    pub bind_address: String,
    pub jwt_secret: String
}

impl AppConfig {
    // 主应用配置
    pub fn from_env() -> Self {
        // 加载.env文件
        dotenv::dotenv().ok();

        Self {
            database_url: std::env::var("DATABASE_URL").expect("DATABASE_URL没有设置"),
            bind_address: std::env::var("BIND_ADDRESS").unwrap_or_else(|_| "0.0.0.0:3000".to_string()),
            jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET没有设置")
        }
    }
    // 集成测试配置
    pub fn for_test() -> Self {
        Self {
            database_url: "mysql://root:sysu@localhost/echat".to_string(),// 测试用的数据库
            bind_address: "0.0.0.0:0".to_string(), // 0 表示随机端口
            jwt_secret: "test-jwt-secret-key".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct Application {
    router: axum::Router,// 路径(状态已经注入到路径中了)
    config: AppConfig,// 应用环境
}

impl Application {
    pub async fn new(config: AppConfig) -> AppResult<Self> {
        // 连接数据库
        let db_pool = MySqlPool::connect(&config.database_url)
            .await
            .map_err(|e| AppError::DatabaseConnectionFailure(e.to_string()))?;
        // 创建应用状态
        let state = AppState::new(db_pool)
            .await
            .map_err(|e| AppError::StateGenerationFailure(e.to_string()))?;

        // 构建路由
        let router = routes::create_routes().with_state(state);

        Ok(Self {
            router,
            config
        })
    }
    // 为测试用例创建实例
    pub async fn new_test() -> AppResult<Self> {
        let config = AppConfig::for_test();
        Self::new(config).await
    }
    // 获取路由(用于HTTP测试)
    pub fn router(&self) -> axum::Router {
        self.router.clone()
    }
    // 获取配置信息
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
    // 启动服务器
    pub async fn run(self) -> AppResult<()> {
        let listener = TcpListener::bind(&self.config.bind_address)
            .await
            .map_err(|e| AppError::ServerStartFailure(e.to_string()))?;

        axum::serve(listener, self.router)
            .await
            .map_err(|e| AppError::ServerStartFailure(e.to_string()))?;

        Ok(())
    }
}

// 快速创建应用实例
pub async fn create_app() -> AppResult<Application> {
    let config = AppConfig::from_env();
    Application::new(config).await
}

// 创建测试应用实例
pub async fn create_test_app() -> AppResult<Application> {
    Application::new_test().await
}