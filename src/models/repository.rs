// src/models/repository.rs
use async_trait::async_trait;
use bb8_redis::RedisConnectionManager;
use bb8_redis::bb8::Pool;
use chrono::NaiveDateTime;

use crate::models::{entities::{FriendRequest, Friends, GroupChat, GroupJoinRequest, GroupMember, GroupMessage, MuteRecord, PrivateChat, PrivateMessage, User, UserOnline}, errors::AppResult};

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
    async fn find_user_by_create_time_range(&self, start:NaiveDateTime, end:NaiveDateTime) -> AppResult<Vec<User>>;
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
    async fn update_request_status(&self, req_id: &str, status: &str, handle_time: NaiveDateTime) -> AppResult<()>;

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
    async fn update_request_status(&self, req_id: &str, status: &str, approver_uid: &str, handle_time: NaiveDateTime) -> AppResult<()>;
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
    async fn find_messages_by_group_and_time_range(&self, gid: &str, start: NaiveDateTime, end: NaiveDateTime) -> AppResult<Vec<GroupMessage>>;
    // 标记消息为已撤回
    async fn mark_message_as_revoked(&self, msg_id: &str) -> AppResult<()>;
//-------------------------消息已读状态管理--------------------------------
    // 标记消息为已读
    async fn mark_message_as_read(&self, msg_id: &str, gid: &str, uid: &str) -> AppResult<()>;
    // 查找消息的已读用户
    async fn find_read_users_by_message(&self, msg_id: &str) -> AppResult<Vec<String>>;
    // 查找用户未读消息
    async fn find_unread_messages_by_user(&self, gid: &str, uid: &str) -> AppResult<Vec<GroupMessage>>;
    // 查找消息已读用户数量
    async fn get_message_read_count(&self, msg_id: &str) -> AppResult<u64>;
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
}