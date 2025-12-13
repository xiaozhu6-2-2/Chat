// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};

// 分离模块导入

// 统一的枚举转换基础设施

/// 枚举转换通用 trait
pub trait EnumConvertible {
    /// 将枚举转换为字符串
    fn to_enum_string(&self) -> String;
    /// 从字符串创建枚举实例
    fn from_enum_string(s: &str) -> Option<Self>
    where
        Self: Sized;
}

/// 处理 Option<枚举> 类型的扩展 trait
pub trait OptionalEnumExt<T: EnumConvertible> {
    /// 将 Option<枚举> 转换为 Option<String>
    fn to_optional_string(&self) -> Option<String>;
    /// 从 Option<String> 创建 Option<枚举>
    fn from_optional_string(s: Option<String>) -> Option<T>;
}

/// 简化实现枚举转换的宏
macro_rules! impl_enum_convertible {
    ($enum_type:ty, { $($variant:ident => $value:expr),* $(,)? }) => {
        impl EnumConvertible for $enum_type {
            fn to_enum_string(&self) -> String {
                match self {
                    $( Self::$variant => $value.to_string(), )*
                }
            }

            fn from_enum_string(s: &str) -> Option<Self> {
                match s {
                    $($value => Some(Self::$variant),)*
                    _ => None,
                }
            }
        }

        impl OptionalEnumExt<$enum_type> for Option<$enum_type> {
            fn to_optional_string(&self) -> Option<String> {
                self.as_ref().map(|e| e.to_enum_string())
            }

            fn from_optional_string(s: Option<String>) -> Option<$enum_type> {
                s.and_then(|s| <$enum_type>::from_enum_string(&s))
            }
        }
    };
}

// 性别枚举
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Gender {
    Male,
    Female,
    Other,
}

// 使用宏实现 Gender 的转换功能
impl_enum_convertible!(Gender, {
    Male => "male",
    Female => "female",
    Other => "other"
});


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
    pub create_time: Option<DateTime<Utc>>,
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
    pub create_time: Option<DateTime<Utc>>,
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

// 使用宏实现 ReqStatus 的转换功能
impl_enum_convertible!(ReqStatus, {
    Pending => "pending",
    Accepted => "accepted",
    Rejected => "rejected",
    Expired => "expired"
});

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

// 群聊加入申请表模型(group_join_request表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupJoinRequest {
    pub req_id: String,// 主键
    pub gid: String,
    pub applicant_uid: String,
    pub approver_uid: Option<String>,
    pub status: ReqStatus,// 枚举pending, accepted, rejected, expired
    pub apply_text: Option<String>,
    pub create_time: Option<DateTime<Utc>>,
    pub handle_time: Option<DateTime<Utc>>,
}

// 角色权限
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type, PartialEq)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum Role {
    Member,
    Admin,
    Owner
}

// 使用宏实现 Role 的转换功能（统一使用小写形式）
impl_enum_convertible!(Role, {
    Member => "member",
    Admin => "admin",
    Owner => "owner"
});


// 群聊成员模型(group_member表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupMember {
    pub uid: String,
    pub gid: String,
    pub role: Role,// 权限
    pub nickname: Option<String>,// 群昵称
    pub level: Option<u8>,// 在群聊的等级
    pub join_time: Option<DateTime<Utc>>,
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

// 使用宏实现 GroupMsgType 的转换功能
impl_enum_convertible!(GroupMsgType, {
    Text => "text",
    Image => "image",
    File => "file",
    Voice => "voice",
    Video => "video",
    Link => "link",
    Emoji => "emoji",
    Annoucement => "annoucement"
});

// 群聊消息表(group_message表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct GroupMessage {
    pub msg_id: String,// 主键
    pub gid: String,
    pub content: String,
    pub sender_uid: String,
    pub send_time: Option<DateTime<Utc>>,
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
    pub start_time: Option<DateTime<Utc>>,
}

// 私聊会话模型(private_chat表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct PrivateChat {
    pub pid: String,
    pub uid1: String,// uid小的
    pub uid2: String,// uid大的
    pub create_time: Option<DateTime<Utc>>,
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

// 使用宏实现 PrivateMsgType 的转换功能
impl_enum_convertible!(PrivateMsgType, {
    Text => "text",
    Image => "image",
    File => "file",
    Voice => "voice",
    Video => "video",
    Link => "link",
    Emoji => "emoji",
    Annoucement => "annoucement"
});

// 私聊消息表(private_message表)
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct PrivateMessage {
    pub msg_id: String,
    pub pid: String,
    pub content: String,
    pub sender_uid: String,
    pub send_time: Option<DateTime<Utc>>,
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
    pub create_time: Option<DateTime<Utc>>,
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
    pub create_time: Option<DateTime<Utc>>,
    pub handle_time: Option<DateTime<Utc>>,
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

// 使用宏实现 FilePrivalege 的转换功能
impl_enum_convertible!(FilePrivalege, {
    Public => "public",
    Friend => "friend",
    Group => "group",
    Private => "private"
});

// 文件状态
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum FileStatus {
    Active,
    Deleted,
    Expired,
}

// 使用宏实现 FileStatus 的转换功能
impl_enum_convertible!(FileStatus, {
    Active => "active",
    Deleted => "deleted",
    Expired => "expired"
});

// 文件引用类型枚举
#[derive(Debug, Clone, Deserialize, Serialize, sqlx::Type)]
#[sqlx(type_name = "VARCHAR", rename_all = "lowercase")]
pub enum ReferenceType {
    Original,  // 原始上传者
    Shared,    // 共享使用者
}

// 使用宏实现 ReferenceType 的转换功能
impl_enum_convertible!(ReferenceType, {
    Original => "original",
    Shared => "shared"
});

// 文件引用表
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FileReference {
    pub reference_id: String,                // 引用ID (雪花ID)
    pub file_hash: String,                   // 文件哈希 (SHA-256)
    pub file_id: String,                     // 关联的文件记录ID
    pub user_uid: String,                    // 用户ID
    pub reference_type: ReferenceType,       // 引用类型
    pub created_at: Option<DateTime<Utc>>,   // 创建时间
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
    pub upload_time: Option<DateTime<Utc>>,
    pub last_access_time: Option<DateTime<Utc>>,
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

// 使用宏实现 AssociationType 的转换功能（确保字符串值与数据库 ENUM 值匹配）
impl_enum_convertible!(AssociationType, {
    PrivateMessage => "private_message",
    GroupMessage => "group_message",
    UserAvatar => "user_avatar",
    GroupAvatar => "group_avatar"
});
// 文件关联表
#[derive(Debug, Clone, Deserialize, Serialize, FromRow)]
pub struct FileAssociation {
    pub association_id: String,
    pub file_id: String,
    pub association_type: AssociationType,
    pub associated_id: String,
    pub created_at: Option<DateTime<Utc>>,
}