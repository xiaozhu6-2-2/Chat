// src/middleware.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::models::Claims;

// JWT验证中间件
pub async fn auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    
    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(std::env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    request.extensions_mut().insert(token_data.claims);

    Ok(next.run(request).await)
}

// WebSocket专用的JWT验证中间件
#[allow(unused_mut)]
pub async fn ws_auth_middleware(
    mut request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    // 从查询参数中提取token
    let token = request.uri()
        .query()
        .and_then(|q| {
            q.split('&')
                .find(|param| param.starts_with("token="))
                .map(|param| param.trim_start_matches("token="))
        });
    
    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;
    
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(std::env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let mut request = request;
    request.extensions_mut().insert(token_data.claims);
    
    Ok(next.run(request).await)
}