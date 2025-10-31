// src/main.rs
mod handlers;
mod models;
mod routes;
mod state;
mod middleware;
mod repository;

use log::info;
// 库模块导入
use tokio::net::TcpListener;
use sqlx::MySqlPool;

// 分离模块导入
use routes::create_routes;
use state::AppState;

#[tokio::main]
async fn main() {
    // 加载.env文件
    dotenv::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set in .env");

    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    info!("Application starting");

    // 创建 MySQL 连接池
    let db_pool = MySqlPool::connect(&db_url)
        .await
        .expect("Failed to create MySQL pool");

    let state = AppState::new(db_pool).await.unwrap();

    // 构建路由(注入状态)
    let app = create_routes().with_state(state);

    // 启动服务器
    let listener = TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}