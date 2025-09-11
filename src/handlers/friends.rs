// 库模块导入
use axum::{
    http::StatusCode,
    Json,
};
use axum::{
    extract::State,
    extract::Path
};
use axum::Extension;

// 分离模块导入
use crate::models::others::{Claims, FriendRequestStatus};
use crate::models::requests::{
    SendFriendRequest, 
    FriendRequestInfo, 
    RespondToFriendRequest, 
};
use crate::models::responses::RegisterResponse;
use crate::models::entities::FriendInfo;
use crate::state::AppState;
use crate::handlers::other::get_username;

// 发送好友请求
pub async fn send_friend_request(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<SendFriendRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let sender_account = claims.sub;
    let receiver_account = payload.receiver_account;

    // 不能添加自己为好友
    if sender_account == receiver_account {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 检查是否已是好友
    let is_friend: Option<i64> = sqlx::query_scalar!(
        r#"SELECT 1 FROM friends 
           WHERE (user_a = ? AND user_b = ?) 
           OR (user_a = ? AND user_b = ?)"#,
        sender_account,
        receiver_account,
        receiver_account,
        sender_account
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if is_friend.is_some() {
        return Ok(Json(RegisterResponse { success: false }));
    }

    // 检查是否已有待处理请求
    let existing_request: Option<i64> = sqlx::query_scalar!(
        r#"SELECT 1 FROM friend_requests 
           WHERE sender_account = ? AND receiver_account = ? AND status = 'PENDING'"#,
        sender_account,
        receiver_account
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if existing_request.is_some() {
        return Ok(Json(RegisterResponse { success: false }));
    }

    // 插入新请求
    sqlx::query!(
        r#"INSERT INTO friend_requests 
           (sender_account, receiver_account, status) 
           VALUES (?, ?, 'PENDING')"#,
        sender_account,
        receiver_account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(RegisterResponse { success: true }))
}

// 列出好友请求（接收和发送的）
pub async fn list_friend_requests(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FriendRequestInfo>>, StatusCode> {
    let account = claims.sub;

    // 查询所有相关请求
    let requests = sqlx::query!(
        r#"SELECT 
            id, 
            sender_account, 
            receiver_account, 
            status as "status: FriendRequestStatus",
            created_at 
        FROM friend_requests 
        WHERE sender_account = ? OR receiver_account = ?"#,
        account,
        account
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 获取用户名信息
    let mut result = Vec::new();
    for req in requests {
        let sender_username = get_username(&state.db_pool, &req.sender_account)
            .await
            .unwrap_or_default();

        result.push(FriendRequestInfo {
            id: req.id,
            sender_account: req.sender_account.clone(),
            sender_username,
            status: req.status.expect("REASON"),
            created_at: req.created_at,
        });
    }

    Ok(Json(result))
}

// 响应好友请求
pub async fn respond_friend_request(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(payload): Json<RespondToFriendRequest>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let account = claims.sub;
    
    // 获取请求
     let request = sqlx::query!(
        r#"SELECT 
            id, 
            sender_account, 
            receiver_account, 
            status as "status: FriendRequestStatus",
            created_at 
        FROM friend_requests WHERE id = ?"#,
        payload.request_id
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    // 验证请求接收者
    if request.receiver_account != account {
        return Err(StatusCode::FORBIDDEN);
    }

    // 更新请求状态
    sqlx::query!(
        "UPDATE friend_requests SET status = ? WHERE id = ?",
        payload.status.to_string(),
        payload.request_id
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 如果接受请求，添加好友关系
    if payload.status == FriendRequestStatus::ACCEPTED {
        // 确保顺序：user_a < user_b
        let (user_a, user_b) = if request.sender_account < request.receiver_account {
            (request.sender_account.clone(), request.receiver_account.clone())
        } else {
            (request.receiver_account.clone(), request.sender_account.clone())
        };

        // 添加好友关系
        sqlx::query!(
            "INSERT INTO friends (user_a, user_b) VALUES (?, ?)",
            user_a,
            user_b
        )
        .execute(&state.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(Json(RegisterResponse { success: true }))
}

// 列出所有好友
pub async fn list_friends(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> Result<Json<Vec<FriendInfo>>, StatusCode> {
    let account = claims.sub;

    // 查询好友
    let friends = sqlx::query_as!(
        FriendInfo,
        r#"SELECT 
            CASE 
                WHEN user_a = ? THEN user_b 
                ELSE user_a 
            END AS account,
            u.username
           FROM friends f
           JOIN user_info u ON 
                (f.user_a = u.account OR f.user_b = u.account) AND u.account != ?
           WHERE user_a = ? OR user_b = ?"#,
        account,
        account,
        account,
        account
    )
    .fetch_all(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(friends))
}

// 删除好友
pub async fn remove_friend(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(friend_account): Path<String>,
) -> Result<Json<RegisterResponse>, StatusCode> {
    let account = claims.sub;

    // 删除好友关系
    let result = sqlx::query!(
        r#"DELETE FROM friends 
           WHERE (user_a = ? AND user_b = ?)
           OR (user_a = ? AND user_b = ?)"#,
        account,
        friend_account,
        friend_account,
        account
    )
    .execute(&state.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() > 0 {
        Ok(Json(RegisterResponse { success: true }))
    } else {
        Ok(Json(RegisterResponse { success: false }))
    }
}