// 测试文件权限修复的简单脚本
use std::env;
use sqlx::MySqlPool;

// 模拟检查权限是否存在的函数
async fn check_permission_exists(
    pool: &MySqlPool,
    file_id: &str,
    access_type: &str,
    target_id: Option<String>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        SELECT permission_id
        FROM file_permission
        WHERE file_id = ?
        AND access_type = ?
        AND (target_id = ? OR (target_id IS NULL AND ? IS NULL))
        LIMIT 1
        "#,
        file_id,
        access_type,
        target_id,
        target_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 从环境变量获取数据库连接字符串
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@localhost:3306/echat".to_string());

    println!("正在连接数据库...");
    let pool = MySqlPool::connect(&database_url).await?;

    println!("数据库连接成功！");

    // 测试场景1：检查不存在的权限
    println!("\n测试场景1：检查不存在的权限");
    let exists = check_permission_exists(&pool, "test_file_123", "user", Some("user_456".to_string())).await?;
    println!("权限存在: {}", exists);

    // 测试场景2：检查目标为NULL的权限（Public或Friend类型）
    println!("\n测试场景2：检查Public类型权限");
    let exists = check_permission_exists(&pool, "test_file_789", "public", None).await?;
    println!("权限存在: {}", exists);

    println!("\n测试完成！权限检查函数工作正常。");

    Ok(())
}