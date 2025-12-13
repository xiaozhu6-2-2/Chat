// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::{
    extract::State,
};
use rsa::traits::PublicKeyParts;
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

use crate::models::repository::UserRepository;
// 分离模块导入
use crate::models::requests::{RegisterRequest, LoginRequest};
use crate::models::responses::{RegisterResponse, LoginResponse, SessionKeyResponse, };
use crate::models::entities::{Gender, OptionalEnumExt, User};
use crate::models::others::{Claims};
use crate::models::errors::{AppError, AppResult};
use crate::state::AppState;
use crate::utils::snowflake;
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

    // 查询有没有account在表中
    let result = state.db_pool.find_user_by_account(&account).await;

    // 如果用户存在则返回DuplicateUser的错误
    if !result.is_err() {
        return Err(AppError::DuplicateUser(result.unwrap().account));
    }

    // 雪花算法实例
    let snowflake_constructor = snowflake::Snowflake::new(1, Some(1_577_836_800_000))?;
    let uid = snowflake_constructor.next_id()?;
    // 插入SQL
    state.db_pool.insert_user(User {
        uid: uid.to_string(),
        account: account,
        password: password_hash,
        username: payload.username,
        gender: Option::<Gender>::from_optional_string(Some(payload.gender)),
        region: Some(payload.region),
        email: None,
        create_time: Some(chrono::Utc::now()),
        avatar: None,
        bio: None,

    }).await?;

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
        Ok(user) => {
            // 生成JWT令牌
            let token = generate_jwt(&account)?;
            
            // 构建响应结构
            Ok(Json(LoginResponse {
                username: user.username,
                account: user.account,
                uid: user.uid,
                token: token
            }))
        },
        Err(e) => Err(e), // 认证失败，返回错误原因(密码错误or用户不存在)
    }
}

// 登录验证逻辑函数(单元测试)
async fn validate_credentials(
    db_pool: &impl UserRepository,
    account: &str,
    password: &str,
) -> AppResult<User> {
    // 从数据库中查询用户信息
    let user = db_pool.find_user_by_account(&account).await?;
    
    // 解析数据库中存储的哈希值为一个PasswordHash对象
    let parsed_hash = PasswordHash::new(&user.password)
        .map_err(|e| AppError::HashFailure(e.to_string()))?;
    
    // 创建argon实例用于验证密码正确性
    let argon2 = Argon2::default();
    // 验证密码正确性
    match argon2.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(user), // 验证成功
        Err(_) => Err(AppError::InvalidPassword),         // 密码不匹配
    }
}

// 私钥解密函数(单元测试)
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

// JWT生成函数(单元测试)
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
) -> Result<Json<SessionKeyResponse>, StatusCode> {
    // 获取公钥
    let public_key = &state.session_key.1;

    // 转化为pkcs#8的格式
    let pk = public_key
        .to_public_key_pem(rsa::pkcs8::LineEnding::LF)
        .map_err(|e| AppError::PubKeyTransitionFailure(e.to_string()))
        .unwrap()
        .to_string();

    Ok(Json(SessionKeyResponse { public_key: pk }))
}