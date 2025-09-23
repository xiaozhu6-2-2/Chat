// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::{
    extract::State,
};
use rsa::traits::PublicKeyParts;
use sqlx::MySqlPool;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher
};
use rand_core::OsRng;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use rsa::pkcs8::EncodePublicKey;
use rsa::Pkcs1v15Encrypt;
use rsa::RsaPrivateKey;
use base64::engine::general_purpose;
use base64::Engine;

// 分离模块导入
use crate::models::requests::{RegisterRequest, LoginRequest};
use crate::models::responses::{RegisterResponse, LoginResponse, SessionKeyRespone, };
use crate::models::entities::{User};
use crate::models::others::{Claims};
use crate::models::errors::{AppError, AppResult};
use crate::state::AppState;

// 注册处理函数
pub async fn register(
    State(state): State<AppState>,// 注入状态
    Json(payload): Json<RegisterRequest>,// 解析为请求结构体
) -> AppResult<Json<RegisterResponse>> {
    // 获取私钥（从全局状态中获取）
    let private_key = &state.session_key.0;

    // 解密账号
    let account = private_key_decrypt(
        private_key,
        &payload.account
    )
    .await?;

    // 解密密码
    let password = private_key_decrypt(
        private_key,
        &payload.password
    ).await?;
    
    info!("auth::register::解密成功");

    // 生成argon2的随机盐值（从rand_core中借用的随机种子生成器）
    let salt = SaltString::generate(&mut OsRng);
    
    // 生成argon2实例，这里使用默认参数
    let argon2 = Argon2::default();
    
    // 生成密码哈希
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::HashFailure(e.to_string()))?
        .to_string();

    // 存储到数据库(error类型是DatabaseFailure)
    // 事务
    let mut tx = state
        .db_pool
        .begin()
        .await?;

    // 查询有没有account在表中
    let result = sqlx::query!(
        "SELECT * FROM user_info WHERE account = ?",
        account
    )
    .fetch_optional(&mut *tx)
    .await?;

    // 如果用户存在则返回DuplicateUser的错误
    if let Some(record) = result {
        return Err(AppError::DuplicateUser(record.account));
    }

    // SQL
    sqlx::query!(
        "INSERT INTO user_info (account, password, username) VALUES (?, ?, ?)",
        account,// 账号明文存储
        password_hash,// 密码存储哈希
        payload.username,// 用户名明文存储
    )
    .execute(&mut *tx)
    .await?;

    // 提交事务
    tx.commit().await?;

    Ok(Json(RegisterResponse { success: true }))
}

// 登录处理函数
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> AppResult<Json<LoginResponse>> {
    // 获取私钥
    let private_key = &state.session_key.0;

    // 解密账号
    let account = private_key_decrypt(
        &private_key,
        &payload.account
    )
    .await?;

    // 解密密码
    let password = private_key_decrypt(
        &private_key,
        &payload.password
    )
    .await?;

    info!("解密后账号如下：{}", account);

    // 用解密后的账号和密码进行登录凭证验证
    match validate_credentials(&state.db_pool, &account, &password).await {
        Ok(username) => {
            // 生成JWT令牌
            let token = generate_jwt(&account)?;
            // 构建响应结构
            Ok(Json(LoginResponse {
                username,
                token,
            }))
        },
        Err(e) => Err(e), // 认证失败，返回错误原因(密码错误or用户不存在)
    }
}

// 登录验证逻辑函数
async fn validate_credentials(
    db_pool: &MySqlPool,
    account: &str,
    password: &str,
) -> AppResult<String> {
    // 事务
    let mut tx = db_pool
        .begin()
        .await?;

    // 从数据库中查询用户信息
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM user_info WHERE account = ?"
    )
    .bind(account)
    .fetch_optional(&mut *tx)
    .await?;

    // 提交事务
    tx.commit().await?;
    
    match user {
        Some(user) => {
            // 解析数据库中存储的哈希值为一个PasswordHash对象
            let parsed_hash = PasswordHash::new(&user.password)
                .map_err(|e| AppError::HashFailure(e.to_string()))?;
            
            // 创建argon实例用于验证密码正确性
            let argon2 = Argon2::default();
            // 验证密码正确性
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(user.username.unwrap()), // 验证成功
                Err(_) => Err(AppError::InvalidPassword),         // 密码不匹配
            }
        }
        None => Err(AppError::UserNotFound(account.to_string())), // 用户不存在
    }
}

// 私钥解密函数
async fn private_key_decrypt(
    private_key : &RsaPrivateKey,
    data : &str
) -> AppResult<String> {
    // 计算模数(字节)
    let key_size_bytes = private_key.n().to_bytes_be().len();

    // 填充方案（比如换行符）
    let padding = Pkcs1v15Encrypt;

    // 将加密字符串转化为BASE64格式
    let ciphertext = general_purpose::STANDARD
        .decode(&data)
        .map_err(|e| AppError::DecryptionFailure(e.to_string()))?;

    // 验证密文长度是否是模数的倍数
    if key_size_bytes != ciphertext.len() {
        return Err(AppError::DecryptionFailure("密文长度不正确".to_string()));
    }

    // 从BASE64中解密密文
    let plain_data = private_key
        .decrypt(padding, &ciphertext)
        .map_err(|e| AppError::DecryptionFailure(e.to_string()))?;

    // 转化为String
    String::from_utf8(plain_data)
        .map_err(|e| AppError::DecryptionFailure(e.to_string()))
} 

// JWT生成函数
fn generate_jwt(account: &str) -> AppResult<String> {
    // 记录当前时间
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| AppError::TokenGenerationFailure(e.to_string()))?
        .as_secs() as usize;
    
    // 计算过期时间：当前时间 + 1小时
    let exp = now + 3600; 
    
    // 创建信息声明
    let claims = Claims {
        sub: account.to_string(),
        exp,
        iat: now,
    };
    
    // 创建token
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(
            std::env::var("JWT_SECRET")
            .map_err(|e| AppError::TokenGenerationFailure(e.to_string())
        )?
        .as_ref())
    )
    .map_err(|e| AppError::TokenGenerationFailure(e.to_string()))?;
    
    Ok(token)
}

// 公钥获取函数
pub async fn get_session_key(
    State(state) : State<AppState>
) -> Result<Json<SessionKeyRespone>, StatusCode> {
    // 获取公钥
    let public_key = &state.session_key.1;

    // 转化为pkcs#8的格式
    let pk = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .unwrap()
        .to_string();

    Ok(Json(SessionKeyRespone { public_key: pk }))
}