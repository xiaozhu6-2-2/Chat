// src/middleware.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::models::{errors::{AppError, AppResult}, others::Claims};

// JWT验证中间件
pub async fn auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    // 在请求头解析Authorization字段
    let token = request.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    
    // 查看该字段的值是否为空，是空的话就返回状态码401
    let token = token.ok_or(AppError::TokenGenerationFailure("Authorization字段为空".to_string()))?;
    
    // 解析该token
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(std::env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::default()
    ).map_err(|e| AppError::TokenGenerationFailure(e.to_string()))?;

    // 将认证信息插入到请求中
    request.extensions_mut().insert(token_data.claims);

    // 交由下一层处理
    Ok(next.run(request).await)
}

// WebSocket专用的JWT验证中间件(token在查询参数中获取)
#[allow(unused_mut)]
pub async fn ws_auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> AppResult<Response> {
    // 从查询参数中提取token
    let token = request.uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|param| param.starts_with("token="))
                .map(|param| param.trim_start_matches("token="))
        });
    
    // 没有token字段返回401状态码
    let token = token.ok_or(AppError::TokenGenerationFailure("查询参数中没有token字段".to_string()))?;
    
    // 解析该token
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(std::env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::default()
    ).map_err(|e| AppError::TokenGenerationFailure(e.to_string()))?;

    // 将认证信息插入请求中
    request.extensions_mut().insert(token_data.claims);
    
    // 交由下一层处理
    Ok(next.run(request).await)
}