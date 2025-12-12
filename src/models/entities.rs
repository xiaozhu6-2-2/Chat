// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::NaiveDateTime;

// 分离模块导入

// 性别枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
}

// 为 Option<Gender> 添加转换扩展
pub trait GenderOptionExt {
    fn to_optional_string(&self) -> Option<String>;
    fn from_optional_string(gender: Option<String>) -> Option<Gender>;
}

impl GenderOptionExt for Option<Gender> {
    fn to_optional_string(&self) -> Option<String> {
        self.as_ref().map(|g| {
            match g {
                Gender::Male => "male".to_string(),
                Gender::Female => "female".to_string(),
                Gender::Other => "other".to_string(),
            }
        })
    }

    fn from_optional_string(gender: Option<String>) -> Option<Gender> {
        match gender.as_deref() {
            Some("male") => Some(Gender::Male),
            Some("female") => Some(Gender::Female),
            Some("other") => Some(Gender::Other),
            Some(_) => None,
            None => None,
        }
    }
}

// 用户表模型(user表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow, PartialEq)]
pub struct User {
    pub uid: String,// 主键（雪花算法）
    pub username: String, // 可相同（昵称）
    pub account: String,// 唯一
    pub password: String,// 非空（哈希加密）
    pub gender: Option<Gender>, 
    pub region: Option<String>,
    pub email: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub avatar: Option<String>,
    pub bio: Option<String>,// 简介
}

// 用户全局在线状态模型(redis)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UserOnline {
    pub account: String,
    pub username: String,
}

// 群聊会话模型(group_chat表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupChat {
    pub gid: String,
    pub group_name: String,
    pub manager_uid: String, // 群主
    pub group_avatar: Option<String>,
    pub group_intro: Option<String>,
    pub create_time: Option<NaiveDateTime>,
}

// 申请状态枚举
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum ReqStatus {
    Pending,
    Accepted,
    Rejected,
    Expired
}

impl std::fmt::Display for ReqStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ReqStatus::Pending => write!(f, "pending"),
            ReqStatus::Accepted => write!(f, "accepted"),
            ReqStatus::Rejected => write!(f, "rejected"),
            ReqStatus::Expired => write!(f, "expired"),
        }
    }
}

// 为 Option<ReqStatus> 添加转换扩展
pub trait ReqStatusOptionExt {
    fn to_optional_string(&self) -> Option<String>;
    fn from_optional_string(status: Option<String>) -> Option<ReqStatus>;
}

impl ReqStatusOptionExt for Option<ReqStatus> {
    fn to_optional_string(&self) -> Option<String> {
        self.as_ref().map(|s| {
            match s {
                ReqStatus::Pending => "pending".to_string(),
                ReqStatus::Accepted => "accepted".to_string(),
                ReqStatus::Rejected => "rejected".to_string(),
                ReqStatus::Expired => "expired".to_string(),
            }
        })
    }

    fn from_optional_string(status: Option<String>) -> Option<ReqStatus> {
        match status.as_deref() {
            Some("pending") => Some(ReqStatus::Pending),
            Some("accepted") => Some(ReqStatus::Accepted),
            Some("rejected") => Some(ReqStatus::Rejected),
            Some("expired") => Some(ReqStatus::Expired),
            Some(_) => None,
            None => None,
        }
    }
}

// 群聊加入申请表模型(group_join_request表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupJoinRequest {
    pub req_id: String,// 主键
    pub gid: String,
    pub applicant_uid: String,
    pub approver_uid: Option<String>,
    pub status: ReqStatus,// 枚举pending, accepted, rejected, expired
    pub apply_text: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub handle_time: Option<NaiveDateTime>,
}

// 角色权限
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Role {
    Member,
    Admin,
    Owner
}


// 群聊成员模型(group_member表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupMember {
    pub uid: String,
    pub gid: String,
    pub role: Role,// 权限
    pub nickname: Option<String>,// 群昵称
    pub level: Option<u8>,// 在群聊的等级
    pub join_time: Option<NaiveDateTime>,
    pub do_not_disturb: Option<i8>,// 免打扰
    pub group_by: Option<String>,// 分组标签
    pub remark: Option<String>,// 备注
    pub is_pinned: Option<i8>,// 置顶状态
}

// 群聊消息类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum GroupMsgType {
    Text,
    Image,
    File,
    Voice,
    Video,
    Link,
    Emoji,
    Annoucement
}

// 群聊消息表(group_message表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupMessage {
    pub msg_id: String,// 主键
    pub gid: String,
    pub content: String,
    pub sender_uid: String,
    pub send_time: Option<NaiveDateTime>,
    pub is_revoked: Option<i8>,// 是否撤回
    #[sqlx(rename = "type")]
    pub msg_type: GroupMsgType,// 枚举类型 text image file voice video link emoji annoucement
    pub mentioned_uids: Option<serde_json::Value>,// Json格式@字段
    pub quote_msg_id: Option<String>,// 消息引用
    pub is_announcement: Option<i8>,// 是否是群公告
}

// 群聊消息已读表(group_message_read表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupMessageRead {
    pub msg_id: String,// 主键
    pub gid: String,
    pub uid: String,// 主键 已读人员
}

// 禁言状态表(mute_record表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct MuteRecord {
    pub ban_id: String,
    pub gid: String,
    pub uid: String,
    pub mute_duration: i64,// 禁言时间
    pub start_time: Option<NaiveDateTime>,
}

// 私聊会话模型(private_chat表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct PrivateChat {
    pub pid: String,
    pub uid1: String,// uid小的
    pub uid2: String,// uid大的
    pub create_time: Option<NaiveDateTime>,
    pub is_pinned_by_uid1: Option<i8>,// uid1置顶状态
    pub is_pinned_by_uid2: Option<i8>,// uid2置顶状态
    pub do_not_disturb_uid1: Option<i8>,
    pub do_not_disturb_uid2: Option<i8>,
}

// 私聊消息类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum PrivateMsgType {
    Text,
    Image,
    File,
    Voice,
    Video,
    Link,
    Emoji,
    Annoucement
}

// 私聊消息表(private_message表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct PrivateMessage {
    pub msg_id: String,
    pub pid: String,
    pub content: String,
    pub sender_uid: String,
    pub send_time: Option<NaiveDateTime>,
    pub is_revoked: Option<i8>,
    pub is_read: Option<i8>,
    #[sqlx(rename = "type")]
    pub mes_type: PrivateMsgType,// 枚举类型 text image file voice video link emoji
}

// 好友表模型(friends表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct Friends {
    pub fid: String,// 主键
    pub uid: String,// 较小的那个
    pub to_uid: String,// 较大的那个
    pub create_time: Option<NaiveDateTime>,
    pub is_blacklist: Option<i8>,// 黑名单，拒收消息
    pub to_is_blacklist: Option<i8>,// 大uid to 小uid的黑名单
    pub remark: Option<String>,// 小uid to 大uid的备注
    pub to_remark: Option<String>, // 大uid to 小uid的备注
    pub group_by: Option<String>,// 小uid to 大uid的分组
    pub to_group_by: Option<String>,// 大uid to 小uid的分组
}

// 好友申请表模型(friend_request表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FriendRequest {
    pub req_id: String,// 主键
    pub sender_uid: String,
    pub receiver_uid: String,
    pub status: ReqStatus,// 枚举pending, accepted, rejected, expired
    pub apply_text: Option<String>,
    pub create_time: Option<NaiveDateTime>,
    pub handle_time: Option<NaiveDateTime>,
}

// 文件访问权限
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum FilePrivalege {
    Public,
    Friend,
    Group,
    Private,
}

// 文件状态
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum FileStatus {
    Active,
    Deleted,
    Expired,
}

// 文件引用类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum ReferenceType {
    Original,  // 原始上传者
    Shared,    // 共享使用者
}

// 文件引用表
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FileReference {
    pub reference_id: String,                // 引用ID (雪花ID)
    pub file_hash: String,                   // 文件哈希 (SHA-256)
    pub file_id: String,                     // 关联的文件记录ID
    pub user_uid: String,                    // 用户ID
    pub reference_type: ReferenceType,       // 引用类型
    pub created_at: Option<NaiveDateTime>,   // 创建时间
}

// 文件信息表
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FileInfo {
    pub file_id: String,
    pub uploader_uid: String,  // 上传者UID
    pub original_name: String,// 原始文件名
    pub file_name: String,// 存储文件名
    pub file_path: String,// 存储路径
    pub file_size: i64,// 文件大小
    pub mime_type: String,// mime类型
    pub file_hash: String,// 文件哈希（用于去重）
    pub access_level: FilePrivalege,
    pub thumbnail_path: Option<String>,// 缩略图路径
    pub upload_time: Option<NaiveDateTime>,
    pub last_access_time: Option<NaiveDateTime>,
    pub download_count: i64,
    pub status: FileStatus,
}

// 文件关联类型
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT")]
pub enum AssociationType {
    #[sqlx(rename = "private_message")]
    PrivateMessage,
    #[sqlx(rename = "group_message")]
    GroupMessage,
    #[sqlx(rename = "user_avatar")]
    UserAvatar,
    #[sqlx(rename = "group_avatar")]
    GroupAvatar,
}
// 文件关联表
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FileAssociation {
    pub association_id: String,
    pub file_id: String,
    pub association_type: AssociationType,
    pub associated_id: String,
    pub created_at: Option<NaiveDateTime>,
}