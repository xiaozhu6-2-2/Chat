// src/models/repository.rs
use async_trait::async_trait;
use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8::Pool;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::{entities::{FriendRequest, Friends, GroupChat, GroupJoinRequest, GroupMember, GroupMessage, MuteRecord, PrivateChat, PrivateMessage, User, UserOnline, FileInfo, FileReference, FileAssociation, FilePrivalege, ReferenceType, AssociationType, FileStatus}, errors::AppResult};

// 对用户表数据操作定义的接口
#[async_trait]
pub trait UserRepository: Send + Sync {
//---------------------------CRUD-------------------------------------
    // 根据uid查找用户
    async fn find_user_by_uid(&self, uid: &str) -> AppResult<User>;
    // 根据账号查找用户
    async fn find_user_by_account(&self, account: &str) -> AppResult<User>;
    // 根据用户名查找用户
    async fn find_user_by_username(&self, username: &str) -> AppResult<Vec<User>>;
    // 根据地区查找用户
    async fn find_user_by_region(&self, region: &str) -> AppResult<Vec<User>>;
    // 根据创建时间查找用户
    async fn find_user_by_create_time_range(&self, start:DateTime<Utc>, end:DateTime<Utc>) -> AppResult<Vec<User>>;
    // 插入用户
    async fn insert_user(&self, user: User) -> AppResult<()>;
    // 保存用户更改
    async fn save_user(&self, user: User) -> AppResult<()>;
    // 删除用户
    async fn delete_user(&self, uid: &str) -> AppResult<()>;
//-----------------------------统计-------------------------------------
    // 根据账号判断用户是否存在
    async fn exists_by_account(&self, account: &str) -> AppResult<bool>;
}

// 好友关系聚合根
#[async_trait]
pub trait FriendshipRepository: Send + Sync {
//-------------------------好友关系管理----------------------------
    // 保存好友关系(若数据库有该关系，则修改；否则插入)
    async fn save_friendship(&self, friendship: Friends) -> AppResult<()>;

    // 根据fid查找好友关系
    async fn find_friendship_by_fid(&self, fid: &str) -> AppResult<Option<Friends>>;
    // 根据两个uid查找好友关系
    async fn find_friendship_by_users(&self, uid1: &str, uid2: &str) -> AppResult<Option<Friends>>;
    // 查找一个用户的好友关系
    async fn find_friendship_by_uid(&self, uid: &str) -> AppResult<Vec<Friends>>;

    // 删除好友关系
    async fn delete_friendship(&self, fid: &str) -> AppResult<()>;

//-------------------------黑名单管理----------------------------
    // 保存记录到黑名单(可以加入黑名单也可以移出黑名单)
    async fn save_blacklist(&self, fid: &str, uid: &str, is_blacklist: bool) -> AppResult<()>;
    // 查找用户黑名单
    async fn find_blacklisted_friends(&self, uid: &str) -> AppResult<Vec<Friends>>;
//-------------------------好友申请管理----------------------------
    // 保存好友申请
    async fn save_friend_request(&self, request: FriendRequest) -> AppResult<()>;
    // 根据req_id查找好友申请
    async fn find_friend_request_by_id(&self, req_id: &str) -> AppResult<Option<FriendRequest>>;
    // 根据接收者查找好友申请
    async fn find_friend_request_by_receiver(&self, receiver_uid: &str) -> AppResult<Vec<FriendRequest>>;
    // 根据发送者查找好友申请
    async fn find_friend_request_by_sender(&self, sender_uid: &str) -> AppResult<Vec<FriendRequest>>;

    // 更新好友申请状态
    async fn update_request_status(&self, req_id: &str, status: &str, handle_time: DateTime<Utc>) -> AppResult<()>;

    // 在事务中处理好友请求接受的所有操作（更新状态、创建好友关系、创建私聊会话）
    async fn accept_friend_request_with_chat(
        &self,
        req_id: &str,
        handle_time: DateTime<Utc>,
        friendship: Friends,
        private_chat: PrivateChat
    ) -> AppResult<()>;

    // 验证私聊消息权限
    async fn validate_private_message_permission(
        &self,
        sender_uid: &str,
        receiver_uid: &str,
    ) -> AppResult<()>;

}

// 群聊管理聚合根
#[async_trait]
pub trait GroupChatRepository: Send + Sync {
//-------------------------群聊基础管理----------------------------
    // 保存群聊
    async fn save_group(&self, group: GroupChat) -> AppResult<()>;
    // 按gid查找群聊
    async fn find_group_by_gid(&self, gid: &str) -> AppResult<Option<GroupChat>>;
    // 按照群主查找群聊
    async fn find_group_by_owner(&self, owner_uid: &str) -> AppResult<Vec<GroupChat>>;
    // 按照群名查找群聊
    async fn find_group_by_name(&self, name: &str) -> AppResult<Vec<GroupChat>>;
    // 删除群聊
    async fn delete_group(&self, gid: &str) -> AppResult<()>;

//-------------------------群聊成员管理----------------------------
    // 加入群聊
    async fn save_member(&self, member: GroupMember) -> AppResult<()>;
    // 查找群成员
    async fn find_member(&self, gid: &str, uid: &str) -> AppResult<Option<GroupMember>>;
    // 查找群聊的群成员列表
    async fn find_members_by_group(&self, gid: &str) -> AppResult<Vec<GroupMember>>;
    // 查找用户的群聊列表
    async fn find_groups_by_user(&self, uid: &str) -> AppResult<Vec<GroupMember>>;
    // 更新用户权限
    async fn update_member_role(&self, role: &str, gid: &str, uid: &str) -> AppResult<()>;
    // 退出群聊
    async fn remove_member(&self, gid: &str, uid: &str) -> AppResult<()>;
//-------------------------禁言管理-------------------------------
    // 禁言用户
    async fn add_mute_record(&self, mute: MuteRecord) -> AppResult<()>;
    // 查找群聊禁言名单
    async fn find_mute_records_by_group(&self, gid: &str) -> AppResult<Vec<MuteRecord>>;
    // 查找用户被禁言记录
    async fn find_mute_records_by_user(&self, gid: &str, uid: &str) -> AppResult<Option<MuteRecord>>;
    // 解除禁言
    async fn remove_mute(&self, ban_id: &str) -> AppResult<()>;
    // 用于清理过期禁言
    async fn find_expired_mute_records(&self) -> AppResult<Vec<MuteRecord>>;
//-------------------------群聊申请管理-------------------------------
    // 保存群聊申请
    async fn save_group_request(&self, request: GroupJoinRequest) -> AppResult<()>;
    // 根据req_id查找群聊申请
    async fn find_group_request_by_id(&self, req_id: &str) -> AppResult<Option<GroupJoinRequest>>;
    // 查找群聊未处理申请
    async fn find_pending_requests_by_group(&self, gid: &str) -> AppResult<Vec<GroupJoinRequest>>;
    // 更新群聊申请状态
    async fn update_request_status(&self, req_id: &str, status: &str, approver_uid: &str, handle_time: DateTime<Utc>) -> AppResult<()>;

    // 验证群聊消息权限
    async fn validate_group_message_permission(
        &self,
        sender_uid: &str,
        group_id: &str,
    ) -> AppResult<()>;
}

// 群聊消息聚合根
#[async_trait]
pub trait GroupMessageRepository: Send + Sync {
//-------------------------群聊消息管理--------------------------------
    // 保存群聊消息
    async fn save_message(&self, msg: GroupMessage) -> AppResult<()>;
    // 按msg_id查找群聊消息
    async fn find_message_by_id(&self, msg_id: &str) -> AppResult<Option<GroupMessage>>;
    // 按gid查找群聊消息
    async fn find_messages_by_group(&self, gid: &str) -> AppResult<Vec<GroupMessage>>;
    // 按gid和时间范围查找群聊消息
    async fn find_messages_by_group_and_time_range(&self, gid: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> AppResult<Vec<GroupMessage>>;
    // 按gid分页查找群聊消息
    async fn find_messages_by_group_with_pagination(&self, gid: &str, limit: i64, offset: i64) -> AppResult<Vec<GroupMessage>>;
    // 获取群聊消息总数
    async fn get_message_count_by_group(&self, gid: &str) -> AppResult<i64>;
    // 标记消息为已撤回
    async fn mark_message_as_revoked(&self, msg_id: &str) -> AppResult<()>;
    //按gid查找查看群公告
    async fn find_announces_by_group(&self, gid: &str) -> AppResult<Vec<GroupMessage>>;
//-------------------------消息已读状态管理--------------------------------
    // 标记消息为已读
    async fn mark_message_as_read(&self, msg_id: &str, gid: &str, uid: &str) -> AppResult<()>;
    // 查找消息的已读用户
    async fn find_read_users_by_message(&self, msg_id: &str) -> AppResult<Vec<String>>;
    // 查找用户未读消息
    async fn find_unread_messages_by_user(&self, gid: &str, uid: &str) -> AppResult<Vec<GroupMessage>>;
    // 获取用户未读消息数量
    async fn get_unread_message_count_by_group(&self, gid: &str, uid: &str) -> AppResult<i32>;
    // 查找消息已读用户数量
    async fn get_message_read_count(&self, msg_id: &str) -> AppResult<u64>;
    // 批量获取多个消息的已读数量
    async fn get_message_read_counts(&self, msg_ids: &[String]) -> AppResult<Vec<(String, i64)>>;
    // 查找群聊的最新消息
    async fn find_latest_message_by_group(&self, gid: &str) -> AppResult<Option<GroupMessage>>;
    // 批量标记群聊消息为已读
    async fn mark_messages_as_read_by_group_and_time(&self, gid: &str, uid: &str, timestamp: DateTime<Utc>) -> AppResult<u64>;
}

// 私聊会话聚合根
#[async_trait]
pub trait PrivateChatRepository: Send + Sync {
//-------------------------私聊会话管理-----------------------
    // 保存私聊会话
    async fn save_chat(&self, chat: PrivateChat) -> AppResult<()>;
    // 按pid查找私聊会话
    async fn find_chat_by_pid(&self, pid: &str) -> AppResult<Option<PrivateChat>>;
    // 查找两个用户的私聊会话
    async fn find_chat_by_users(&self, uid1: &str, uid2: &str) -> AppResult<Option<PrivateChat>>;
    // 查找用户的私聊会话
    async fn find_chats_by_user(&self, uid: &str) -> AppResult<Vec<PrivateChat>>;
    // 更新用户置顶状态
    async fn update_pin_status(&self, pid: &str, uid: &str, is_pinned: bool) -> AppResult<()>;
//-------------------------私聊消息管理-----------------------
    // 保存私聊消息
    async fn save_message(&self, msg: PrivateMessage) -> AppResult<()>;
    // 按msg_id查找私聊消息
    async fn find_message_by_id(&self, msg_id: &str) -> AppResult<Option<PrivateMessage>>;
    // 按pid查找私聊消息
    async fn find_messages_by_chat(&self, pid: &str) -> AppResult<Vec<PrivateMessage>>;
    // 标记消息为已读
    async fn mark_message_as_read(&self, msg_id: &str) -> AppResult<()>;
    // 标记消息为撤回
    async fn mark_message_as_revoked(&self, msg_id: &str) -> AppResult<()>;
    // 查找未读消息
    async fn find_unread_message_by_chat(&self, pid: &str, uid: &str) -> AppResult<Vec<PrivateMessage>>;
    // 获取未读消息数量
    async fn get_unread_message_count_by_chat(&self, pid: &str, uid: &str) -> AppResult<i32>;
    // 查找会话的最新消息
    async fn find_latest_message_by_chat(&self, pid: &str) -> AppResult<Option<PrivateMessage>>;
    // 获取私聊会话的消息总数
    async fn get_message_count_by_chat(&self, pid: &str) -> AppResult<i64>;
    // 批量标记私聊消息为已读
    async fn mark_messages_as_read_by_chat_and_time(&self, pid: &str, uid: &str, timestamp: DateTime<Utc>) -> AppResult<u64>;
}

// 在线状态聚合根
#[async_trait]
pub trait OnlineRepository: Send + Sync {
    // 在线状态上线
    async fn user_online(
        redis_pool : &Pool<RedisConnectionManager>,
        infomation : UserOnline,
        group_ids : &[String]
    ) -> AppResult<()>;

    // 在线状态下线
    async fn user_offline(
        redis_pool : &Pool<RedisConnectionManager>,
        account : String,
        group_ids : &[String]
    ) -> AppResult<()>;

    // 更新心跳
    async fn update_heartbeat(
        redis_pool : &Pool<RedisConnectionManager>,
        group_ids : &[String]
    ) -> AppResult<()>;

    // 批量查询用户在线状态
    async fn batch_check_online_status(
        redis_pool : &Pool<RedisConnectionManager>,
        accounts: &[String]
    ) -> AppResult<Vec<String>>;

    // 获取群聊在线成员
    async fn get_group_online_members(
        redis_pool : &Pool<RedisConnectionManager>,
        gid: &str
    ) -> AppResult<Vec<String>>;
}

// 文件管理聚合根
#[async_trait]
pub trait FileRepository: Send + Sync {
    //-------------------------文件基础管理----------------------------
    // 保存文件信息
    async fn save_file(&self, file: crate::models::entities::FileInfo) -> AppResult<()>;
    // 根据file_id查找文件
    async fn find_file_by_id(&self, file_id: &str) -> AppResult<Option<crate::models::entities::FileInfo>>;
    // 根据文件哈希查找文件（用于去重检查）
    async fn find_file_by_hash(&self, file_hash: &str) -> AppResult<Option<crate::models::entities::FileInfo>>;
    // 根据上传者查找文件
    async fn find_files_by_uploader(&self, upload_uid: &str) -> AppResult<Vec<crate::models::entities::FileInfo>>;
    // 根据MIME类型查找文件
    async fn find_files_by_mime_type(&self, mime_type: &str) -> AppResult<Vec<crate::models::entities::FileInfo>>;
    // 根据访问权限查找文件
    async fn find_files_by_access_level(&self, access_level: &str) -> AppResult<Vec<crate::models::entities::FileInfo>>;
    // 更新文件访问时间
    async fn update_file_access_time(&self, file_id: &str) -> AppResult<()>;
    // 增加文件下载次数
    async fn increment_download_count(&self, file_id: &str) -> AppResult<()>;
    // 更新文件状态
    async fn update_file_status(&self, file_id: &str, status: &str) -> AppResult<()>;
    // 删除文件（逻辑删除）
    async fn delete_file(&self, file_id: &str) -> AppResult<()>;
    // 物理删除过期文件
    async fn cleanup_expired_files(&self) -> AppResult<u64>;

    //-------------------------文件引用管理----------------------------
    // 创建文件引用
    async fn create_file_reference(&self, reference: crate::models::entities::FileReference) -> AppResult<()>;
    // 查找用户的文件引用
    async fn find_file_references_by_user(&self, user_uid: &str) -> AppResult<Vec<crate::models::entities::FileReference>>;
    // 查找文件的所有引用
    async fn find_file_references_by_file(&self, file_id: &str) -> AppResult<Vec<crate::models::entities::FileReference>>;
    // 删除文件引用
    async fn delete_file_reference(&self, reference_id: &str) -> AppResult<()>;
    // 检查用户是否可以访问文件
    async fn check_file_access_permission(&self, file_hash: &str, user_uid: &str) -> AppResult<bool>;

    //-------------------------文件关联管理----------------------------
    // 创建文件关联
    async fn create_file_association(&self, association: crate::models::entities::FileAssociation) -> AppResult<()>;
    // 根据关联类型和ID查找关联的文件
    async fn find_file_associations_by_type_and_id(&self, association_type: &str, associated_id: &str) -> AppResult<Vec<crate::models::entities::FileAssociation>>;
    // 根据文件ID查找所有关联
    async fn find_file_associations_by_file(&self, file_id: &str) -> AppResult<Vec<crate::models::entities::FileAssociation>>;
    // 删除文件关联
    async fn delete_file_association(&self, association_id: &str) -> AppResult<()>;
    // 批量删除关联（如删除消息时）
    async fn delete_file_associations_by_type_and_id(&self, association_type: &str, associated_id: &str) -> AppResult<u64>;

    //-------------------------文件共享管理----------------------------
    // 创建文件共享（为其他用户创建引用）
    async fn share_file_to_user(&self, file_hash: &str, from_uid: &str, to_uid: &str) -> AppResult<()>;
    // 查找用户可访问的文件（包括自己的和共享的）
    async fn find_accessible_files_by_user(&self, user_uid: &str) -> AppResult<Vec<(crate::models::entities::FileInfo, crate::models::entities::ReferenceType)>>;
    // 撤销文件共享
    async fn revoke_file_access(&self, file_hash: &str, owner_uid: &str, target_uid: &str) -> AppResult<()>;

    //-------------------------文件统计和分析----------------------------
    // 获取用户的文件使用统计
    async fn get_user_file_statistics(&self, user_uid: &str) -> AppResult<FileStatistics>;
    // 获取系统文件使用统计
    async fn get_system_file_statistics(&self) -> AppResult<SystemFileStatistics>;
    // 查找大文件
    async fn find_large_files(&self, size_threshold: i64) -> AppResult<Vec<crate::models::entities::FileInfo>>;
    // 查找久未访问的文件
    async fn find_inactive_files(&self, days_threshold: i64) -> AppResult<Vec<crate::models::entities::FileInfo>>;
}

// 文件统计信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FileStatistics {
    pub total_files: u64,
    pub total_size: i64,
    pub by_type: Vec<(String, u64)>,  // (mime_type, count)
    pub by_access_level: Vec<(String, u64)>,  // (access_level, count)
    pub upload_count_today: u64,
    pub download_count_total: u64,
}

// 系统文件统计信息
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SystemFileStatistics {
    pub total_files: u64,
    pub total_size: i64,
    pub active_files: u64,
    pub expired_files: u64,
    pub unique_files: u64,  // 去重后的文件数量
    pub storage_efficiency: f64,  // (去重后大小 / 原始总大小) * 100
}