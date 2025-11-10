// src/main.rs
use echat;

use log::{info, error};

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();
    info!("Application 创建ing");

    match echat::create_app().await {
        Ok(app) => {
            info!("应用创建成功，启动服务器");
            // 运行服务器
            if let Err(e) = app.run().await {
                error!("服务器运行失败: {}", e);
            }
        }
        Err(e) => {
            error!("应用创建失败: {}", e);
        }
    }
}