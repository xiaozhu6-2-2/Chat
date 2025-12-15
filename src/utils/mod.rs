pub mod snowflake;
pub mod group_listener_manager;
pub mod trans_logic;

use crate::models::errors::{AppError, AppResult};
use std::sync::OnceLock;

static SNOWFLAKE: OnceLock<snowflake::Snowflake> = OnceLock::new();

/// 获取全局雪花ID生成器实例
fn get_snowflake() -> AppResult<&'static snowflake::Snowflake> {
    SNOWFLAKE.get_or_try_init(|| {
        snowflake::Snowflake::new(1, Some(0))
    })
}

/// 生成雪花ID的便捷函数
pub fn generate_snowflake_id() -> AppResult<String> {
    let snowflake = get_snowflake()?;
    Ok(snowflake.next_id()?.to_string())
}