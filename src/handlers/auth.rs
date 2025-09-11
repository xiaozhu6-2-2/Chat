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
use std::error::Error;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier, SaltString},
    Argon2, PasswordHasher
};
use rand_core::OsRng;
use jsonwebtoken::{encode, EncodingKey, Header};
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::info;
use rsa::pkcs1::EncodeRsaPublicKey;
use rsa::Pkcs1v15Encrypt;
use rsa::RsaPrivateKey;
use base64::engine::general_purpose;
use base64::Engine;

// 分离模块导入
use crate::models::requests::{RegisterRequest, LoginRequest};
use crate::models::responses::{RegisterResponse, LoginResponse, SessionKeyRespone, };
use crate::models::entities::{User};
use crate::models::others::{Claims};

use crate::state::AppState;

// 注册处理函数
pub async fn register(
    State(state): State<AppState>,// 注入状态
    Json(payload): Json<RegisterRequest>,// 解析为请求结构体
) -> Result<Json<RegisterResponse>, StatusCode> {
    // 获取私钥
    let private_key = &state.session_key.0;

    // 解密账号密码
    let account = private_key_decrypt(private_key, &payload.account).await?;
    let password = private_key_decrypt(private_key, &payload.password).await?;
    
    info!("解密前账号：{}", payload.account);
    info!("解密后账号：{}", account);

    // 生成随机盐值
    let salt = SaltString::generate(&mut OsRng);
    
    // 配置Argon2参数
    let argon2 = Argon2::default();
    
    // 生成密码哈希
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    // 存储到数据库
    sqlx::query!(
        "INSERT INTO user_info (account, password, username) VALUES (?, ?, ?)",
        account,
        password_hash,
        payload.username,
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterResponse { success: true }))
}

// 登录处理函数
pub async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
    // 获取私钥
    let private_key = &state.session_key.0;

    // 解密账号和密码
    let account = private_key_decrypt(&private_key, &payload.account).await?;
    let password = private_key_decrypt(&private_key, &payload.password).await?;

    info!("解密后账号如下：{}", account);

    match validate_credentials(&state.db_pool, &account, &password).await {
        Ok(Some(username)) => {
            // 生成JWT令牌
            let token = generate_jwt(&account)
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                
            Ok(Json(LoginResponse {
                username,
                token,
            }))
        }
        Ok(None) => Err(StatusCode::UNAUTHORIZED), // 认证失败， 返回401
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),// 服务器内部错误， 返回500
    }
}

// 登录验证逻辑函数
async fn validate_credentials(
    db_pool: &MySqlPool,
    account: &str,
    password: &str,
) -> Result<Option<String>, Box<dyn Error>> {
    // 从数据库中查询用户信息
    let user = sqlx::query_as::<_, User>(
        "SELECT * FROM user_info WHERE account = ?"
    )
    .bind(account)
    .fetch_optional(db_pool)
    .await?;

    match user {
        Some(user) => {
            // 验证密码哈希
            let parsed_hash = PasswordHash::new(&user.password)
                .map_err(|_| "密码哈希解析失败")?;
                
            let argon2 = Argon2::default();
            match argon2.verify_password(password.as_bytes(), &parsed_hash) {
                Ok(_) => Ok(user.username), // 验证成功
                Err(_) => Ok(None),         // 密码不匹配
            }
        }
        None => Ok(None), // 用户不存在
    }
}

// 私钥解密函数
async fn private_key_decrypt(private_key : &RsaPrivateKey, data : &str) -> Result<String, StatusCode> {
    // 计算模数(字节)
    let key_size_bytes = private_key.n().to_bytes_be().len();

    // 填充方案
    let padding = Pkcs1v15Encrypt;

    // 将加密字符串转化为BASE64格式
    let ciphertext = general_purpose::STANDARD.decode(&data)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // 验证密文长度是否是模数的倍数
    if key_size_bytes != ciphertext.len() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 解密密文
    let plain_data = private_key.decrypt(padding, &ciphertext)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    String::from_utf8(plain_data)
        .map_err(|_| StatusCode::BAD_REQUEST)
} 

// 公钥获取函数
pub async fn get_session_key(
    State(state) : State<AppState>
) -> Result<Json<SessionKeyRespone>, StatusCode> {
    let public_key = &state.session_key.1;
    let pk = public_key
        .to_pkcs1_pem(rsa::pkcs1::LineEnding::LF)
        .unwrap()
        .to_string();
    Ok(Json(SessionKeyRespone { public_key: pk }))
}

// JWT生成函数
fn generate_jwt(account: &str) -> Result<String, Box<dyn Error>> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .as_secs() as usize;
    
    let exp = now + 3600; // 1小时有效期
    
    let claims = Claims {
        sub: account.to_string(),
        exp,
        iat: now,
    };
    
    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(std::env::var("JWT_SECRET")?.as_ref())
    )?;
    
    Ok(token)
}