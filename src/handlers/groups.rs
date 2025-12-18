use axum::Extension;
use axum::{extract::State, Json};

use crate::models::others::Claims;
use crate::models::requests::{BanningMemberRequest, CreateGroupRequest, DisbandGroupRequest, GetAnnouncementsRequest, GetBanStatusRequest, GetMembersRequest, GroupAvatarRequest, GroupProfileRequest, GroupRequestListRequest, GroupRequestRequest, GroupRespondRequest, KickMemberRequest, LeaveGroupRequest, MemberSettingRequest, RemoveMuteRequest, RemovingingAdminRequest, SettingAdminRequest, SettingGroupRequest, TransferOwnershipRequest};
use crate::models::responses::{AnnouncementItem, BanningMemberResponse, CreateGroupResponse, DisbandGroupResponse, GetAnnouncementsResponse, GetBanStatusResponse, GetMembersResponse, GetRequestItem, GetRequestListResponse, GroupAvatarResponse, GroupListItem, GroupListResponse, GroupProfileResponse, GroupRequestItem, GroupRequestListResponse, GroupRequestResponse, GroupRespondResponse, KickMemberResponse, LeaveGroupResponse, MemberItem, MemberSettingResponse, RemoveMuteResponse, RemovingAdminResponse, SettingAdminResponse, SettingGourpResponse, TransferOwnershipResponse};
use crate::models::entities::{ReqStatus, OptionalEnumExt, EnumConvertible, AssociationType, AccessTarget, AccessLevel};
use crate::models::{errors::{AppResult, AppError}, responses::{SearchGroupResponse, SearchGroupItem}, responses::GroupCardResponse, requests::SearchGroupRequest, requests::GroupCardRequest};
use crate::models::repository::{GroupChatRepository, GroupMessageRepository, UserRepository, FileRepository};
use crate::state::AppState;
use log::{info, error};

pub async fn create_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<CreateGroupRequest>,
) -> AppResult<Json<CreateGroupResponse>>{
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 生成群组ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let gid = snowflake.next_id()?.to_string();

    // 创建群组实体
    let group = crate::models::entities::GroupChat {
        gid: gid.clone(),
        group_name: payload.group_name.clone(),
        manager_uid: user.uid.clone(),
        group_avatar: payload.avatar.clone(),
        group_intro: payload.group_intro.clone(),
        create_time: None,  // 让数据库自动生成时间
    };

    // 保存群组到数据库
    state.db_pool.save_group(group).await?;

    // 创建群主成员记录
    let manager_member = crate::models::entities::GroupMember {
        uid: user.uid.clone(),
        gid: gid.clone(),
        role: crate::models::entities::Role::Owner,
        nickname: None,
        level: Some(1),
        join_time: None,  // 让数据库自动生成时间
        do_not_disturb: Some(0),
        group_by: None,
        remark: None,
        is_pinned: Some(0),
    };

    // 保存群主成员信息
    state.db_pool.save_member(manager_member).await?;

    // 新增：检查创建者是否在线，如果是则启动群聊监听
    if let Some(tx) = state.connection_pool.get(&user.account) {
        // 创建者在线，启动监听
        if let Err(e) = state.group_task_manager.add_listener(
            user.uid.clone(),
            user.account.clone(),
            gid.clone(),
            tx.clone(),
            state.broadcast_pool.clone()
        ).await {
            error!("为创建者启动群聊 {} 监听失败: {}", gid, e);
        } else {
            info!("为创建者启动群聊 {} 监听成功", gid);
        }
    }

    // 从数据库查询群组信息以获取创建时间
    let created_group = state.db_pool.find_group_by_gid(&gid).await?
        .ok_or_else(|| AppError::DatabaseFailure(sqlx::Error::RowNotFound))?;

    // 返回响应
    Ok(Json(CreateGroupResponse {
        gid,
        created_at: created_group.create_time
            .map(|dt| dt.timestamp())
            .unwrap_or(0),
    }))
}

pub async fn search_group(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<SearchGroupRequest>,
) -> AppResult<Json<SearchGroupResponse>> {
    // 验证分页参数
    if payload.offset < 0 || payload.limit <= 0 || payload.limit > 100 {
        return Err(AppError::BadRequest("请求参数错误".to_string()));
    }

    // 设置分页参数
    let limit = payload.limit;
    let offset = payload.offset;

    let search_results = if payload.query.is_empty() {
        // 如果查询为空，返回空结果
        Vec::new()
    } else {
        // 使用多种搜索方式组合查询
        let mut all_results = Vec::new();
        let mut seen_gids = std::collections::HashSet::new();

        // 1. GID 精准搜索（如果查询是数字且长度合适）
        let is_numeric_query = payload.query.chars().all(|c| c.is_ascii_digit());
        if is_numeric_query && payload.query.len() > 8 {
            if let Ok(Some(group)) = state.db_pool.find_group_by_gid(&payload.query).await {
                seen_gids.insert(group.gid.clone());
                all_results.push(group);
            }
        }

        // 2. 群组名称模糊搜索（始终执行）
        let search_pattern = format!("%{}%", payload.query);
        if let Ok(mut name_results) = state.db_pool.find_group_by_name(&search_pattern).await {
            // 过滤掉已经通过 GID 搜索找到的群组，避免重复
            name_results.retain(|group| !seen_gids.contains(&group.gid));
            all_results.extend(name_results);
        }

        all_results
    };

    // 实现分页
    let total_groups = search_results.len() as i64;

    // 计算总页数（向上取整）
    let total_pages = if total_groups == 0 {
        0
    } else {
        (total_groups / limit) + 1
    };

    // 检查页码是否超出范围
    if total_pages > 0 && offset >= total_pages {
        return Err(AppError::PageOutOfRange {
            page: offset,
            total_pages,
        });
    }

    let start_index = (offset * limit) as usize;
    let end_index = std::cmp::min(start_index + limit as usize, total_groups as usize);

    let paginated_groups = if start_index < total_groups as usize {
        search_results[start_index..end_index].to_vec()
    } else {
        Vec::new()
    };

    // 转换为响应格式
    let response_groups: Vec<SearchGroupItem> = paginated_groups
        .into_iter()
        .map(|group| SearchGroupItem {
            gid: group.gid,
            group_name: group.group_name,
            avatar: group.group_avatar,
            bio: group.group_intro,
        })
        .collect();

    Ok(Json(SearchGroupResponse {
        total_pages,
        current_page: offset,
        total_items: total_groups,
        groups: response_groups,
    }))
}

pub async fn get_group_card(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<GroupCardRequest>,
) -> AppResult<Json<GroupCardResponse>> {
    // 获取群组信息
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 构建群组卡片响应（基本信息）
    Ok(Json(GroupCardResponse {
        gid: group.gid,
        group_name: group.group_name,
        manager_uid: group.manager_uid,
        avatar: group.group_avatar,
        group_intro: group.group_intro,
        created_at: group.create_time.map(|dt| dt.timestamp()),
    }))
}

pub async fn get_group_profile(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupProfileRequest>,
) -> AppResult<Json<GroupProfileResponse>> {
    // 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 首先验证用户是否是该群组的成员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;

    // 如果用户不是群组成员，返回错误
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        });
    }

    let member_info = member.unwrap();

    // 获取群组信息
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 构建响应
    Ok(Json(GroupProfileResponse {
        gid: group.gid,
        group_name: group.group_name,
        manager_uid: group.manager_uid,
        avatar: group.group_avatar.unwrap_or_default(),
        group_intro: group.group_intro.unwrap_or_default(),
        created_at: group.create_time
            .map(|dt| dt.timestamp()),
        do_not_disturb: member_info.do_not_disturb.unwrap_or(0) == 1,
        is_pinned: member_info.is_pinned.unwrap_or(0) == 1,
        remark: member_info.remark,
        nickname: member_info.nickname,
        join_time: member_info.join_time
            .map(|dt| dt.timestamp())
            .unwrap_or(0),
    }))
}

pub async fn get_group_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
) -> AppResult<Json<GroupListResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 获取用户加入的所有群组
    let group_members = state.db_pool.find_groups_by_user(&user.uid).await?;

    // 4. 获取每个群组的详细信息并构建响应
    let mut group_list = Vec::new();

    for member in group_members {
        // 查找群组信息
        if let Some(group) = state.db_pool.find_group_by_gid(&member.gid).await? {
            let group_item = GroupListItem {
                gid: group.gid,
                group_name: group.group_name,
                avatar: group.group_avatar.unwrap_or_default(),
                bio: group.group_intro.unwrap_or_default(),
            };
            group_list.push(group_item);
        }
    }

    // 5. 计算总数
    let total = group_list.len() as i64;

    // 6. 返回响应
    Ok(Json(GroupListResponse {
        groups: group_list,
        total,
    }))
}

pub async fn send_group_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupRequestRequest>,
) -> AppResult<Json<GroupRequestResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证群组是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 4. 验证用户是否已经是群组成员
    let existing_member = state.db_pool.find_member(&payload.gid, &user.uid).await?;
    if existing_member.is_some() {
        return Err(AppError::BadRequest("用户已经是群组成员".to_string()));
    }

    // 5. 检查是否已有待处理的申请（数据库层面已过滤为待处理状态）
    let existing_requests = state.db_pool.find_pending_requests_by_group(&payload.gid).await?;
    for req in existing_requests {
        if req.applicant_uid == user.uid {
            return Err(AppError::BadRequest("已经有待处理的加入申请".to_string()));
        }
    }

    // 6. 生成申请ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let req_id = snowflake.next_id()?.to_string();

    // 7. 创建群聊申请记录
    let group_request = crate::models::entities::GroupJoinRequest {
        req_id: req_id.clone(),
        gid: payload.gid.clone(),
        applicant_uid: user.uid.clone(),
        approver_uid: None,
        status: ReqStatus::Pending,
        apply_text: Some(payload.apply_text.clone()),
        create_time: None,
        handle_time: None,
    };

    // 8. 保存申请到数据库
    state.db_pool.save_group_request(group_request).await?;

    // 9. 从数据库查询申请信息以获取创建时间
    let saved_request = state.db_pool.find_group_request_by_id(&req_id).await?
        .ok_or_else(|| AppError::DatabaseFailure(sqlx::Error::RowNotFound))?;

    // 10. 构建响应
    Ok(Json(GroupRequestResponse {
        req_id,
        gid: payload.gid,
        group_name: group.group_name.clone(),
        group_avatar: group.group_avatar.clone().unwrap_or_default(),
        sender_uid: user.uid.clone(),
        apply_text: payload.apply_text,
        create_time: saved_request.create_time
            .map(|dt| dt.timestamp())
            .unwrap_or(0),
        status: Some(saved_request.status).to_optional_string().unwrap_or_default(),
    }))
}

pub async fn get_request_list(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>
) -> AppResult<Json<GetRequestListResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 查找用户的所有群聊申请记录（包括待处理、已接受、已拒绝等）
    let user_requests = state.db_pool.find_requests_by_user(&user.uid).await?;

    // 4. 转换为响应格式
    let mut request_items: Vec<GetRequestItem> = Vec::new();

    for req in user_requests {
        // 获取群组信息
        let group_info = state.db_pool.find_group_by_gid(&req.gid).await.ok().and_then(|x| x);
        let group_name = group_info.as_ref().map(|g| &g.group_name).cloned().unwrap_or_default();
        let group_avatar = group_info.as_ref().and_then(|g| g.group_avatar.clone()).unwrap_or_default();

        let request_item = GetRequestItem {
            req_id: req.req_id,
            gid: req.gid,
            group_name,
            group_avatar,
            sender_uid: req.applicant_uid,
            apply_text: req.apply_text,
            create_time: req.create_time
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            status: Some(req.status).to_optional_string().unwrap_or_default(),
        };

        request_items.push(request_item);
    }

    // 5. 计算总数
    let total = request_items.len() as i64;

    // 6. 返回响应
    Ok(Json(GetRequestListResponse {
        requests: request_items,
        total,
    }))
}

pub async fn group_requests(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupRequestListRequest>,
) -> AppResult<Json<GroupRequestListResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证群组是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 4. 验证请求者是否是群主或管理员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 检查是否是群主或管理员
    let role_str = member.role.to_enum_string();
    if role_str != "owner" && role_str != "admin" {
        return Err(AppError::BadRequest("只有群主或管理员可以查看申请列表".to_string()));
    }

    // 3. 获取该群组的所有待处理申请（数据库层面已过滤）
    let pending_requests = state.db_pool.find_pending_requests_by_group(&payload.gid).await?;

    // 4. 转换为响应格式
    let mut request_items: Vec<GroupRequestItem> = Vec::new();

    for req in pending_requests {
        // 获取发送者信息
        let sender_info = state.db_pool.find_user_by_uid(&req.applicant_uid).await.ok().and_then(|x| Some(x));
        let sender_name = sender_info.as_ref().map(|u| &u.username).cloned().unwrap_or_default();
        let sender_avatar = sender_info.as_ref().and_then(|u| u.avatar.clone()).unwrap_or_default();

        let request_item = GroupRequestItem {
            req_id: req.req_id,
            gid: req.gid,
            group_name: group.group_name.clone(),
            group_avatar: group.group_avatar.clone().unwrap_or_default(),
            sender_uid: req.applicant_uid,
            sender_name,
            sender_avatar,
            apply_text: req.apply_text,
            create_time: req.create_time
                .map(|dt| dt.timestamp())
                .unwrap_or(0),
            status: Some(req.status).to_optional_string().unwrap_or_default(),
        };

        request_items.push(request_item);
    }

    // 5. 计算总数
    let total = request_items.len() as i64;

    // 6. 返回响应
    Ok(Json(GroupRequestListResponse {
        requests: request_items,
        total,
    }))
}

pub async fn handle_group_request(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupRespondRequest>,
) -> AppResult<Json<GroupRespondResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 查找申请记录
    let request = state.db_pool.find_group_request_by_id(&payload.req_id).await?
        .ok_or_else(|| AppError::NotFound(format!("申请{}不存在", payload.req_id)))?;

    // 4. 验证申请是否还是待处理状态
    if request.status != ReqStatus::Pending {
        return Err(AppError::BadRequest("申请已经被处理".to_string()));
    }

    // 5. 验证 action 参数
    let status = match payload.action.as_str() {
        "accept" => ReqStatus::Accepted,
        "reject" => ReqStatus::Rejected,
        _ => return Err(AppError::BadRequest("无效的action参数,必须是 'accept' 或 'reject'".to_string())),
    };

    // 6. 获取当前时间
    let now = chrono::Utc::now();

    // 7. 更新申请状态
    state.db_pool.update_request_status(
        &payload.req_id,
        Some(status).to_optional_string().unwrap_or_default().as_str(),
        &user.uid,
        now
    ).await?;

    // 8. 如果是接受申请，将用户加入群组
    if payload.action == "accept" {
        let member = crate::models::entities::GroupMember {
            uid: request.applicant_uid.clone(),
            gid: request.gid.clone(),
            role: crate::models::entities::Role::Member,
            nickname: None,
            level: Some(1),
            join_time: Some(now),
            do_not_disturb: Some(0),
            group_by: None,
            remark: None,
            is_pinned: Some(0),
        };

        state.db_pool.save_member(member).await?;

        // 新增：为新加入的用户启动群聊监听（如果在线）
        if let Ok(applicant_user) = state.db_pool.find_user_by_uid(&request.applicant_uid).await {
            if let Some(conn_pool) = state.connection_pool.get(&applicant_user.account) {
                // 用户在线，启动监听
                if let Err(e) = state.group_task_manager.add_listener(
                    request.applicant_uid.clone(),
                    applicant_user.account.clone(),
                    request.gid.clone(),
                    conn_pool.clone(),
                    state.broadcast_pool.clone()
                ).await {
                    error!("为用户 {} 启动群聊 {} 监听失败: {}", request.applicant_uid, request.gid, e);
                } else {
                    info!("为用户 {} 启动群聊 {} 监听成功", request.applicant_uid, request.gid);
                }
            }
        }
    }

    // 9. 返回成功响应（空响应体，只返回状态码）
    Ok(Json(GroupRespondResponse {
        success: true,
    }))
}

pub async fn leave_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<LeaveGroupRequest>,
) -> AppResult<Json<LeaveGroupResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证群组是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 4. 查找用户在群组中的成员信息
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 5. 检查用户角色
    let role_str = member.role.to_enum_string();
    if role_str == "owner" {
        // 群主不能直接退出群组
        return Err(AppError::BadRequest(
            "群主不能直接退出群组，请先转让群主身份或解散群组".to_string()
        ));
    }
    // 管理员和普通成员可以正常退出

    // 6. 删除成员记录
    state.db_pool.remove_member(&payload.gid, &user.uid).await?;

    // 新增：取消用户的群聊监听任务
    if let Err(e) = state.group_task_manager.remove_listener(&user.uid, &payload.gid).await {
        error!("取消用户 {} 群聊 {} 监听失败: {}", user.uid, payload.gid, e);
    } else {
        info!("取消用户 {} 群聊 {} 监听成功", user.uid, payload.gid);
    }

    // 7. 返回成功响应
    Ok(Json(LeaveGroupResponse {
        success: true,
    }))
}

pub async fn kick_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<KickMemberRequest>,
) -> AppResult<Json<KickMemberResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let operator = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证群组是否存在
    let _group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 4. 查找管理员（执行踢人操作的用户）的成员信息
    let admin_member = state.db_pool.find_member(&payload.gid, &operator.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: operator.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 5. 验证管理员权限
    let admin_role_str = admin_member.role.to_enum_string();
    if admin_role_str == "member" {
        // 普通成员不能踢人
        return Err(AppError::InsufficientPermission("普通成员没有踢人权限".to_string()));
    }
    // 群主可以踢出任何人，管理员需要额外检查

    // 6. 查找被踢用户的成员信息
    let target_member = state.db_pool.find_member(&payload.gid, &payload.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: payload.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 7. 权限检查
    // 7.1 用户不能踢自己
    if payload.uid == operator.uid {
        return Err(AppError::BadRequest("不能踢出自己，请使用退出群聊接口".to_string()));
    }

    // 7.2 普通管理员不能踢出群主
    let target_role_str = target_member.role.to_enum_string();
    if admin_role_str == "admin" && target_role_str == "owner" {
        return Err(AppError::InsufficientPermission("管理员不能踢出群主".to_string()));
    }

    // 8. 执行踢人操作
    state.db_pool.remove_member(&payload.gid, &payload.uid).await?;

    // 新增：取消被踢用户的群聊监听任务
    if let Err(e) = state.group_task_manager.remove_listener(&payload.uid, &payload.gid).await {
        error!("取消被踢用户 {} 群聊 {} 监听失败: {}", payload.uid, payload.gid, e);
    } else {
        info!("取消被踢用户 {} 群聊 {} 监听成功", payload.uid, payload.gid);
    }

    // 9. 返回响应
    Ok(Json(KickMemberResponse {
        success: true,
    }))
}

pub async fn disband_group(
    State(state): State<AppState>,
    Extension(_claims): Extension<Claims>,
    Json(payload): Json<DisbandGroupRequest>,
) -> AppResult<Json<DisbandGroupResponse>> {
    // 1. 从JWT token中获取用户账号
    let user_account = &_claims.sub;

    // 2. 通过账号查找用户信息，获取用户ID
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证群组是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?
        .ok_or_else(|| AppError::NotFound(format!("群组{}不存在", payload.gid)))?;

    // 4. 验证请求者是否是群主
    if group.manager_uid != user.uid {
        return Err(AppError::InsufficientPermission("只有群主可以解散群聊".to_string()));
    }

    // 新增：清理该群组所有监听任务
    // 获取群组的所有成员（在删除前获取）
    let members = match state.db_pool.find_members_by_group(&payload.gid).await {
        Ok(members) => members,
        Err(e) => {
            error!("获取群组 {} 成员列表失败: {}", payload.gid, e);
            Vec::new()
        }
    };

    // 5. 删除群组
    state.db_pool.delete_group(&payload.gid).await?;

    // 取消每个成员的监听任务
    for member in members {
        if let Err(e) = state.group_task_manager.remove_listener(&member.uid, &payload.gid).await {
            error!("取消用户 {} 群聊 {} 监听失败: {}", member.uid, payload.gid, e);
        } else {
            info!("取消用户 {} 群聊 {} 监听成功", member.uid, payload.gid);
        }
    }

    // 清理群聊广播频道
    if let Some((_gid, _channel)) = state.broadcast_pool.remove(&payload.gid) {
        info!("清理群组 {} 的广播频道成功", payload.gid);
    }

    // 7. 返回成功响应
    Ok(Json(DisbandGroupResponse {
        success: true,
    }))
}

pub async fn member_set(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<MemberSettingRequest>,
) -> AppResult<Json<MemberSettingResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证用户是否是该群组的成员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;

    // 如果用户不是群组成员，返回错误
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        });
    }

    // 4. 更新成员设置
    let do_not_disturb = if payload.do_not_disturb { 1 } else { 0 };
    let is_pinned = if payload.is_pinned { 1 } else { 0 };
    let remark = if payload.remark.is_empty() { None } else { Some(payload.remark) };
    let nickname = if payload.nickname.is_empty() { None } else { Some(payload.nickname) };

    let updated_member = crate::models::entities::GroupMember {
        uid: user.uid.clone(),
        gid: payload.gid.clone(),
        role: member.unwrap().role, // 保持原有角色
        nickname,
        level: Some(1),
        join_time: None, // 不修改加入时间
        do_not_disturb: Some(do_not_disturb),
        group_by: None,
        remark,
        is_pinned: Some(is_pinned),
    };

    // 5. 保存更新
    state.db_pool.save_member(updated_member).await?;

    // 6. 返回成功响应
    Ok(Json(MemberSettingResponse {
        success: true,
    }))
}

pub async fn set_group(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SettingGroupRequest>,
) -> AppResult<Json<SettingGourpResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 2. 查找群聊是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group = group.ok_or_else(|| AppError::NotFound(format!("群聊 {} 不存在", payload.gid)))?;

    // 3. 查找用户在群中的角色
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;
    let member = member.ok_or_else(|| AppError::NotGroupMember {
        uid: user.uid.clone(),
        gid: payload.gid.clone(),
    })?;

    // 4. 验证权限（只有群主和管理员可以修改群聊信息）
    let role_str = member.role.to_enum_string();
    if role_str != "owner" && role_str != "admin" {
        return Err(AppError::InsufficientPermission("只有群主和管理员可以修改群聊信息".to_string()));
    }

    // 5. 检查信息是否有变化
    let mut has_changes = false;

    if group.group_name != payload.group_name {
        has_changes = true;
    }

    if let Some(current_avatar) = group.group_avatar {
        if current_avatar != payload.group_avater {
            has_changes = true;
        }
    } else if !payload.group_avater.is_empty() {
        has_changes = true;
    }

    if let Some(current_intro) = group.group_intro {
        if current_intro != payload.group_intro {
            has_changes = true;
        }
    } else if !payload.group_intro.is_empty() {
        has_changes = true;
    }

    if !has_changes {
        return Err(AppError::BadRequest("修改的群聊信息与当前信息相同".to_string()));
    }

    // 6. 更新群聊信息
    let updated_group = crate::models::entities::GroupChat {
        gid: payload.gid.clone(),
        group_name: payload.group_name.clone(),
        manager_uid: group.manager_uid,
        group_avatar: Some(payload.group_avater.clone()),
        group_intro: Some(payload.group_intro.clone()),
        create_time: group.create_time, // 保持原有创建时间
    };

    state.db_pool.save_group(updated_group).await?;

    // 7. 返回成功响应
    Ok(Json(SettingGourpResponse {
        success: true,
    }))
}

pub async fn set_group_avatar(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GroupAvatarRequest>,
) -> AppResult<Json<GroupAvatarResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 2. 验证用户是否是群组成员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;
    let member = member.ok_or_else(|| AppError::NotGroupMember {
        uid: user.uid.clone(),
        gid: payload.gid.clone(),
    })?;

    // 3. 验证用户权限（群主或管理员）
    let role_str = member.role.to_enum_string();
    if role_str != "owner" && role_str != "admin" {
        return Err(AppError::InsufficientPermission("只有群主和管理员可以设置群头像".to_string()));
    }

    // 4. 验证文件ID有效性
    let has_permission = state.db_pool.verify_file_permission(
        &payload.group_avater,
        &user.uid,
        AccessLevel::Download,  // 至少需要下载权限
    ).await?;

    if !has_permission {
        return Err(AppError::InsufficientPermission("您没有权限使用该文件作为群头像".to_string()));
    }

    // 4.1 验证文件类型是否为图像
    let file_metadata = state.db_pool.find_file_metadata_by_id(&payload.group_avater).await?
        .ok_or_else(|| AppError::NotFound("文件不存在".to_string()))?;

    // 检查文件类型是否为图像
    let is_image = file_metadata.file_type.starts_with("image/");

    if !is_image {
        return Err(AppError::BadRequest("只能使用图像文件作为群头像".to_string()));
    }

    // 5. 查询当前群组信息
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group = group.ok_or_else(|| AppError::NotFound(format!("群聊 {} 不存在", payload.gid)))?;

    // 5.1 处理旧头像（如果存在）
    if let Some(old_avatar_file_id) = &group.group_avatar {
        // 删除旧的群组头像关联
        state.db_pool.batch_delete_associations_by_target(
            AssociationType::GroupAvatar,
            &payload.gid
        ).await?;

        // 软删除旧头像文件
        state.db_pool.soft_delete_file(old_avatar_file_id).await?;
    }

    // 6. 创建新的文件关联
    state.db_pool.create_file_association(
        &payload.group_avater,
        AssociationType::GroupAvatar,
        &payload.gid,
        &user.uid
    ).await?;

    // 7. 设置群组文件权限
    // 为新头像文件授权群组成员查看权限
    state.db_pool.grant_file_permission(
        &payload.group_avater,
        AccessTarget::Group,
        Some(payload.gid.clone()),
        AccessLevel::Download,  // 群组成员可查看头像
        &user.uid,
        None  // 永不过期
    ).await?;

    // 8. 更新群组信息
    let updated_group = crate::models::entities::GroupChat {
        gid: group.gid.clone(),
        group_name: group.group_name.clone(),
        manager_uid: group.manager_uid.clone(),
        group_avatar: Some(payload.group_avater.clone()),  // 更新头像
        group_intro: group.group_intro.clone(),
        create_time: group.create_time,  // 保持原有创建时间
    };

    // 保存更新
    state.db_pool.save_group(updated_group).await?;

    // 9. 返回响应
    Ok(Json(GroupAvatarResponse {
        success: true,
    }))
}

pub async fn get_announcements(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GetAnnouncementsRequest>,
) -> AppResult<Json<GetAnnouncementsResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证用户是否是该群组的成员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;

    // 如果用户不是群组成员，返回错误
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        });
    }

    // 4. 使用现有的 find_announces_by_group 方法获取群公告
    let announcements_db = state.db_pool.find_announces_by_group(&payload.gid).await?;

    // 5. 转换查询结果为 AnnouncementItem 结构体，并按时间倒序排列
    let mut announcements: Vec<AnnouncementItem> = announcements_db
        .into_iter()
        .map(|msg| {
            // 解析 mentioned_uids JSON
            let mentioned_uids: Vec<String> = if let Some(mentioned) = msg.mentioned_uids {
                // 将 serde_json::Value 转换为 Vec<String>
                match mentioned {
                    serde_json::Value::Array(arr) => {
                        arr.into_iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    }
                    _ => Vec::new(),
                }
            } else {
                Vec::new()
            };

            AnnouncementItem {
                msg_id: msg.msg_id,
                content: msg.content,
                sender_uid: msg.sender_uid,
                send_time: msg.send_time.map(|t| t.timestamp()).unwrap_or(0),
                mentioned_uids,
                quote_msg_id: msg.quote_msg_id.unwrap_or_default(),
            }
        })
        .collect();

    // 按发送时间倒序排序（最新的在前）
    announcements.sort_by(|a, b| b.send_time.cmp(&a.send_time));

    // 6. 获取总数
    let total = announcements.len() as i32;

    // 7. 返回响应
    Ok(Json(GetAnnouncementsResponse {
        announcements,
        total,
    }))
}

pub async fn get_members(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GetMembersRequest>,
) -> AppResult<Json<GetMembersResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证用户是否是该群组的成员
    let member = state.db_pool.find_member(&payload.gid, &user.uid).await?;

    // 如果用户不是群组成员，返回错误
    if member.is_none() {
        return Err(AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        });
    }

    // 4. 获取群组的所有成员
    let group_members = state.db_pool.find_members_by_group(&payload.gid).await?;

    // 5. 转换为 MemberItem 格式
    let mut members: Vec<MemberItem> = Vec::new();

    for group_member in group_members {
        // 查找用户详细信息
        match state.db_pool.find_user_by_uid(&group_member.uid).await {
            Ok(user_info) => {
                // 转换角色为字符串
                let role_str = group_member.role.to_enum_string();

                // 获取群昵称，如果没有则使用用户名
                let nickname = group_member.nickname.unwrap_or_else(|| user_info.username.clone());

                // 获取头像，如果没有则使用默认头像或空字符串
                let avatar = user_info.avatar.unwrap_or_else(|| "".to_string());

                let member_item = MemberItem {
                    role: role_str,
                    uid: group_member.uid,
                    username: user_info.username,
                    avatar,
                    nickname,
                };

                members.push(member_item);
            }
            Err(_) => {
                // 如果找不到用户信息，跳过该成员
                continue;
            }
        }
    }

    // 6. 获取总数
    let total = members.len() as i32;

    // 7. 返回响应
    Ok(Json(GetMembersResponse {
        members,
        total,
    }))
}

pub async fn transfer_ownership(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<TransferOwnershipRequest>,
) -> AppResult<Json<TransferOwnershipResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证不能转让给自己
    if user.uid == payload.uid {
        return Err(AppError::BadRequest("不能转让群主给自己".to_string()));
    }

    // 4. 查找群聊信息，验证群主身份
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group = group.ok_or_else(|| AppError::NotFound(format!("群聊 {} 不存在", payload.gid)))?;

    // 验证当前用户确实是群主
    if group.manager_uid != user.uid {
        return Err(AppError::InsufficientPermission("您不是该群的群主".to_string()));
    }

    // 6. 查找被转让者的成员信息
    let target_member = state.db_pool.find_member(&payload.gid, &payload.uid).await?;
    let target_member = target_member.ok_or_else(|| AppError::NotFound(format!("用户 {} 不是该群成员", payload.uid)))?;

    // 7. 使用事务进行转让操作
    let mut tx = state.db_pool.begin().await?;

    // 7.1 将原群主降级为普通成员
    sqlx::query!(
        "UPDATE group_member SET role = ? WHERE gid = ? AND uid = ?",
        "Member",
        payload.gid,
        user.uid
    )
    .execute(&mut *tx)
    .await?;

    // 7.2 将被转让者升级为群主
    sqlx::query!(
        "UPDATE group_member SET role = ? WHERE gid = ? AND uid = ?",
        "Owner",
        payload.gid,
        payload.uid
    )
    .execute(&mut *tx)
    .await?;

    // 7.3 更新群聊的群主信息
    sqlx::query!(
        "UPDATE group_chat SET manager_uid = ? WHERE gid = ?",
        payload.uid,
        payload.gid
    )
    .execute(&mut *tx)
    .await?;

    // 提交事务
    tx.commit().await?;

    // 8. 返回成功响应
    Ok(Json(TransferOwnershipResponse {
        success: true,
    }))
}

pub async fn set_admin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<SettingAdminRequest>,
) -> AppResult<Json<SettingAdminResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 查找群聊信息，验证群是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group = group.ok_or_else(|| AppError::NotFound(format!("群聊 {} 不存在", payload.gid)))?;

    // 4. 验证当前用户是否是群主（只有群主可以设置管理员）
    if group.manager_uid != user.uid {
        return Err(AppError::InsufficientPermission("只有群主可以设置管理员".to_string()));
    }

    // 5. 查找目标用户的成员信息
    let target_member = state.db_pool.find_member(&payload.gid, &payload.uid).await?;
    let target_member = target_member.ok_or_else(|| AppError::NotFound(format!("用户 {} 不是该群成员", payload.uid)))?;

    // 6. 验证目标用户必须是普通成员
    let target_role_str = target_member.role.to_enum_string();
    if target_role_str != "member" {
        return Err(AppError::BadRequest("只能将普通成员设置为管理员".to_string()));
    }

    // 7. 验证不能设置群主为管理员
    if payload.uid == group.manager_uid {
        return Err(AppError::BadRequest("不能设置群主为管理员".to_string()));
    }

    // 8. 使用事务更新用户角色
    let mut tx = state.db_pool.begin().await?;

    // 将目标用户设置为管理员
    sqlx::query!(
        "UPDATE group_member SET role = ? WHERE gid = ? AND uid = ?",
        "Admin",
        payload.gid,
        payload.uid
    )
    .execute(&mut *tx)
    .await?;

    // 提交事务
    tx.commit().await?;

    // 9. 返回成功响应
    Ok(Json(SettingAdminResponse {
        success: true,
    }))
}

pub async fn remove_admin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RemovingingAdminRequest>,
) -> AppResult<Json<RemovingAdminResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 查找群聊信息，验证群是否存在
    let group = state.db_pool.find_group_by_gid(&payload.gid).await?;
    let group = group.ok_or_else(|| AppError::NotFound(format!("群聊 {} 不存在", payload.gid)))?;

    // 4. 验证当前用户是否是群主（只有群主可以移除管理员）
    if group.manager_uid != user.uid {
        return Err(AppError::InsufficientPermission("只有群主可以移除管理员".to_string()));
    }

    // 5. 查找目标用户的成员信息
    let target_member = state.db_pool.find_member(&payload.gid, &payload.uid).await?;
    let target_member = target_member.ok_or_else(|| AppError::NotFound(format!("用户 {} 不是该群成员", payload.uid)))?;

    // 6. 验证目标用户必须是管理员
    let target_role_str = target_member.role.to_enum_string();
    if target_role_str != "admin" {
        return Err(AppError::BadRequest("只能移除管理员权限".to_string()));
    }

    // 7. 验证不能移除群主的管理员权限（群主本身就是 Owner）
    if payload.uid == group.manager_uid {
        return Err(AppError::BadRequest("不能移除群主的管理员权限".to_string()));
    }

    // 8. 使用事务更新用户角色为普通成员
    let mut tx = state.db_pool.begin().await?;

    // 将目标用户设置为普通成员
    sqlx::query!(
        "UPDATE group_member SET role = ? WHERE gid = ? AND uid = ?",
        "Member",
        payload.gid,
        payload.uid
    )
    .execute(&mut *tx)
    .await?;

    // 提交事务
    tx.commit().await?;

    // 9. 返回成功响应
    Ok(Json(RemovingAdminResponse {
        success: true,
    }))
}

pub async fn get_ban_status(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<GetBanStatusRequest>,
) -> AppResult<Json<GetBanStatusResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 查找用户在群中的禁言记录
    let mute_record = state.db_pool.find_mute_records_by_user(&payload.gid, &user.uid).await?;

    // 4. 判断是否被禁言
    if let Some(record) = mute_record {
        // 检查禁言是否已过期
        let now = chrono::Utc::now();

        // 获取开始时间，如果不存在则使用当前时间
        let start_time = record.start_time.unwrap_or(now);

        // 如果 mute_duration 为 -1，表示永久禁言
        if record.mute_duration == -1 {
            Ok(Json(GetBanStatusResponse {
                is_banned: true,
                expired: -1,
            }))
        } else if record.mute_duration == 0 {
            // 0 表示未禁言
            Ok(Json(GetBanStatusResponse {
                is_banned: false,
                expired: 0,
            }))
        } else {
            // 计算结束时间
            let end_time = start_time + chrono::Duration::seconds(record.mute_duration);

            if end_time > now {
                // 仍在禁言期，返回剩余时间戳
                let remain_timestamp = end_time.timestamp();
                Ok(Json(GetBanStatusResponse {
                    is_banned: true,
                    expired: remain_timestamp,
                }))
            } else {
                // 禁言已过期
                Ok(Json(GetBanStatusResponse {
                    is_banned: false,
                    expired: 0,
                }))
            }
        }
    } else {
        // 没有禁言记录
        Ok(Json(GetBanStatusResponse {
            is_banned: false,
            expired: 0,
        }))
    }
}

pub async fn ban_member(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<BanningMemberRequest>,
) -> AppResult<Json<BanningMemberResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证操作者是否是群主或管理员
    let operator_member = state.db_pool.find_member(&payload.gid, &user.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 只有群主和管理员可以禁言
    let operator_role_str = operator_member.role.to_enum_string();
    if operator_role_str == "member" {
        return Err(AppError::InsufficientPermission("只有群主和管理员可以禁言成员".to_string()));
    }

    // 4. 验证被禁言用户是否在群中
    let target_member = state.db_pool.find_member(&payload.gid, &payload.uid).await?
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 不是该群成员", payload.uid)))?;

    // 5. 检查权限（管理员不能禁言群主）
    let target_role_str = target_member.role.to_enum_string();
    if operator_role_str == "admin" && target_role_str == "owner" {
        return Err(AppError::InsufficientPermission("管理员不能禁言群主".to_string()));
    }

    // 6. 管理员不能禁言其他管理员
    if operator_role_str == "admin" && target_role_str == "admin" {
        return Err(AppError::InsufficientPermission("管理员不能禁言其他管理员".to_string()));
    }

    // 7. 解析禁言时长
    let mute_duration = payload.time;

    // 8. 生成禁言ID
    let snowflake = crate::utils::snowflake::Snowflake::new(1, None)?;
    let ban_id = snowflake.next_id()?.to_string();

    // 9. 创建禁言记录
    let mute_record = crate::models::entities::MuteRecord {
        ban_id,
        gid: payload.gid.clone(),
        uid: payload.uid.clone(),
        mute_duration,
        start_time: Some(chrono::Utc::now()),
    };

    // 10. 保存禁言记录
    state.db_pool.add_mute_record(mute_record).await?;

    // 11. 返回成功响应
    Ok(Json(BanningMemberResponse {
        success: true,
    }))
}

pub async fn remove_mute_admin(
    State(state): State<AppState>,
    Extension(claims): Extension<Claims>,
    Json(payload): Json<RemoveMuteRequest>,
) -> AppResult<Json<RemoveMuteResponse>> {
    // 1. 从 claims 中获取用户账号
    let user_account = &claims.sub;

    // 2. 通过账号查找用户信息，获取 uid
    let user = state.db_pool.find_user_by_account(user_account).await?;

    // 3. 验证操作者是否是群主或管理员
    let operator_member = state.db_pool.find_member(&payload.gid, &user.uid).await?
        .ok_or_else(|| AppError::NotGroupMember {
            uid: user.uid.clone(),
            gid: payload.gid.clone(),
        })?;

    // 只有群主和管理员可以解除禁言
    let operator_role_str = operator_member.role.to_enum_string();
    if operator_role_str == "member" {
        return Err(AppError::InsufficientPermission("只有群主和管理员可以解除禁言".to_string()));
    }

    // 4. 查找目标用户的禁言记录
    let mute_record = state.db_pool.find_mute_records_by_user(&payload.gid, &payload.uid).await?;

    // 验证是否有禁言记录
    let mute_record = mute_record
        .ok_or_else(|| AppError::NotFound(format!("用户 {} 在该群中没有禁言记录", payload.uid)))?;

    // 5. 解除禁言（将 mute_duration 设置为 0）
    state.db_pool.remove_mute(&mute_record.ban_id).await?;

    // 6. 返回成功响应
    Ok(Json(RemoveMuteResponse {
        success: true,
    }))
}