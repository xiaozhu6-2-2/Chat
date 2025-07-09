// src/middleware.rs
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use crate::models::Claims;

pub async fn auth_middleware(
    request: Request<Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let token = request.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    
    let token = token.ok_or(StatusCode::UNAUTHORIZED)?;
    
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(std::env::var("JWT_SECRET").unwrap().as_ref()),
        &Validation::default()
    ).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(next.run(request).await)
}