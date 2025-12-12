use axum::Extension;
use axum::{extract::State, Json};

use crate::models::others::Claims;
use crate::models::{errors::{AppError, AppResult}, responses::FriendsOnlineResponse, responses::GroupOnlineResponse, responses::OnlineFriendItem, requests::GroupOnlineRequest};
use crate::models::repository::{UserRepository, FriendshipRepository, OnlineRepository, GroupChatRepository};
use crate::repository::OnlineRepository::OnlineManager;
use crate::state::AppState;

pub async fn get_friends_online(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<FriendsOnlineResponse>> {
    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 根据当前用户的 uid 获取所有好友关系
    let friendships = state.db_pool.find_friendship_by_uid(&current_user.uid).await?;

    // 4. 收集所有好友的 uid
    let mut friend_uids = Vec::new();
    for friendship in friendships {
        // 判断当前用户是 uid 还是 to_uid
        let friend_uid = if current_user.uid == friendship.uid {
            friendship.to_uid.clone()
        } else {
            friendship.uid.clone()
        };
        friend_uids.push(friend_uid);
    }

    // 5. 查询好友的详细信息
    let mut friends_info = Vec::new();
    for friend_uid in &friend_uids {
        match state.db_pool.find_user_by_uid(friend_uid).await {
            Ok(user) => {
                friends_info.push(user);
            }
            Err(_) => {
                // 忽略查询失败的用户
                continue;
            }
        }
    }

    // 6. 提取所有好友的 account 用于查询在线状态
    let friend_accounts: Vec<String> = friends_info.iter()
        .map(|user| user.account.clone())
        .collect();

    // 7. 使用 OnlineRepository 批量查询在线状态
    let online_accounts = OnlineManager::batch_check_online_status(&state.redis_pool, &friend_accounts).await?;

    // 8. 构建响应，只包含在线的好友
    let mut online_friends = Vec::new();
    for user in friends_info {
        if online_accounts.contains(&user.account) {
            let online_friend = OnlineFriendItem {
                user_id: user.uid,
                username: user.username,
                avatar: user.avatar.unwrap_or_default(),
                status: "online".to_string(),
                last_seen_at: None, // 暂时不实现
            };
            online_friends.push(online_friend);
        }
    }

    // 9. 返回响应
    let total = online_friends.len() as i64;
    let response = FriendsOnlineResponse {
        online_friends,
        total,
    };

    Ok(Json(response))
}

pub async fn get_group_online(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupOnlineRequest>,
) -> AppResult<Json<GroupOnlineResponse>> {
    // 1. 从 claims 中提取当前用户的 sub (account)
    let current_account = &claims.sub;

    // 2. 通过 account 查找当前用户
    let current_user = state.db_pool.find_user_by_account(current_account).await?;

    // 3. 检查用户是否在群聊中
    let member = state.db_pool.find_member(&payload.gid, &current_user.uid).await?;
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: current_user.uid.clone(),
            gid: payload.gid.clone()
        });
    }

    // 4. 从 Redis 获取群聊在线成员账号列表
    let online_accounts = OnlineManager::get_group_online_members(&state.redis_pool, &payload.gid).await?;

    // 5. 根据在线账号查询用户详细信息
    let mut online_group_members = Vec::new();
    for account in online_accounts {
        match state.db_pool.find_user_by_account(&account).await {
            Ok(user) => {
                let member_item = crate::models::responses::OnlineGroupMemberItem {
                    user_id: user.uid,
                    username: user.username,
                    avatar: user.avatar.unwrap_or_default(),
                    status: "online".to_string(),
                    last_seen_at: None, // 暂时不实现
                };
                online_group_members.push(member_item);
            }
            Err(_) => {
                // 忽略查询失败的用户
                continue;
            }
        }
    }

    // 6. 返回响应
    let total = online_group_members.len() as i64;
    let response = GroupOnlineResponse {
        online_group_members,
        total,
    };

    Ok(Json(response))
}