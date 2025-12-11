use chrono::NaiveDateTime;
// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};

// 注册响应结构体
#[derive(Serialize, Deserialize)]
pub struct RegisterResponse {
    pub success: bool,
}

// 登录响应模型
#[derive(Serialize, Deserialize)]
pub struct LoginResponse {
    pub username: String, 
    pub account: String,
    pub uid: String,
    pub token: String,     // JWT令牌
}

// 获取公钥响应模型
#[derive(Serialize, Deserialize)]
pub struct SessionKeyResponse {
    pub public_key: String
}

// 获取用户信息响应模型
#[derive(Serialize, Deserialize)]
pub struct UserInfoResponse {
    pub uid: String,
    pub account: String,
    pub username: String,
    pub gender: Option<String>,        
    pub region: Option<String>,        
    pub email: Option<String>,         
    pub create_time: Option<NaiveDateTime>,  
    pub avatar: Option<String>,        
    pub bio: Option<String>,         
}

// 用户信息更新响应模型
#[derive(Serialize, Deserialize)]
pub struct UserInfoUpdateResponse {
    pub success: bool,
}

// 获取用户资料响应模型
#[derive(Serialize, Deserialize)]
pub struct FetchProfileResponse {
    pub uid: String,
    pub account: String,
    pub username: String,
    pub gender: Option<String>,        
    pub region: Option<String>,        
    pub email: Option<String>,
    pub avatar: Option<String>,        
    pub bio: Option<String>, 
}

// 搜索用户条目模型
#[derive(Serialize, Deserialize)]
pub struct SearchUserItem {
    pub uid: String,
    pub username: String,
    pub gender: Option<String>,
    pub avatar: Option<String>,
    pub bio: Option<String>,
}

// 搜索用户响应模型
#[derive(Serialize, Deserialize)]
pub struct SearchUserResponse {
    pub total_pages: i64, // 总页数
    pub current_page: i64, // 当前页码
    pub total_items: i64,  // 总条目数
    pub users: Vec<SearchUserItem>,
}

// 好友资料响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendProfileResponse {
    pub fid: String,// 好友关系id
    pub uid: String,// 好友id
    pub account: String,// 好友账号
    pub username: String,// 好友用户名
    pub remark: String,// 对好友的备注
    pub group_by: String,// 对好友的分组
    pub is_blacklisted: bool,// 对好友的黑名单状态
    pub created_at: Option<NaiveDateTime>,// 好友账号的创建时间
    pub bio: Option<String>,// 好友的简介
    pub avatar: Option<String>,// 好友的头像
    pub gender: Option<String>,// 好友的性别
    pub region: Option<String>,// 好友的地区
    pub email: Option<String>,// 好友的联系方式
}

// 好友列表项模型
#[derive(Serialize, Deserialize)]
pub struct FriendItem {
    pub fid: String,
    pub uid: String,
    pub username: String,
    pub remark: String,
    pub group_by: String,
    pub is_blacklisted: bool,
    pub created_at: Option<NaiveDateTime>,
    pub bio: Option<String>,
    pub avatar: Option<String>,
}

// 好友列表响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendListResponse {
    pub total: i64,
    pub friends: Vec<FriendItem>,
    pub blacklist: Vec<FriendItem>,
}

// 好友请求响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendRequestResponse {
    pub req_id: String,
    pub sender_uid: String,
    pub receiver_uid: String,
    pub apply_text: Option<String>,
    pub create_time: String,
    pub status: Option<String>,
}

// 回复好友请求响应模型
#[derive(Serialize, Deserialize)]
pub struct RespondFriendRequestResponse {
    pub uid: String,// 用户id
    pub fid: String,// 好友关系id
}

// 好友请求项
#[derive(Serialize, Deserialize)]
pub struct FriendRequestItem {
    pub req_id: String,
    pub sender_uid: String,
    pub apply_text: Option<String>,
    pub create_time: Option<String>,
    pub status: String,
}

// 好友请求列表响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendRequestListResponse {
    pub total: i64,
    pub requests: Vec<FriendRequestItem>,// 自己的请求
    pub receives: Vec<FriendRequestItem>,// 别人的请求
}

// 删除好友响应模型
#[derive(Serialize, Deserialize)]
pub struct RemoveFriendResponse {

}

// 更新好友备注响应模型
#[derive(Serialize, Deserialize)]
pub struct UpdateFriendRemarkResponse {

}

// 更新好友黑名单响应模型
#[derive(Serialize, Deserialize)]
pub struct UpdateFriendBlacklistResponse {

}


// 创建群组响应模型
#[derive(Serialize, Deserialize)]
pub struct CreateGroupResponse {
    pub gid: String,
    pub groupname: String,
    pub manager_uid: String,
    pub avatar: String,
    pub groupintro: String,
    pub created_at: String,
}

// 搜索群组项
#[derive(Serialize, Deserialize)]
pub struct SearchGroupItem {
    pub gid: String,
    pub group_name: String,
    pub avatar: Option<String>,
    pub bio: Option<String>,
}

// 搜索群组响应模型
#[derive(Serialize, Deserialize)]
pub struct SearchGroupResponse {
    pub total_pages: i64,     // 总页数
    pub current_page: i64,   // 当前页码
    pub total_items: i64,    // 总条目数
    pub groups: Vec<SearchGroupItem>,
}

// 群组名片响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupCardResponse {
    pub gid: String,
    pub group_name: String,
    pub manager_uid: String,
    pub avatar: Option<String>,
    pub group_intro: Option<String>,
    pub created_at: Option<NaiveDateTime>,
}

// 群组资料响应模型（仅群成员可用）
#[derive(Serialize, Deserialize)]
pub struct GroupProfileResponse {
    pub gid: String,
    pub group_name: String,
    pub manager_uid: String,
    pub avatar: String,
    pub group_intro: String,
    pub created_at: String,
    pub do_not_disturb: bool,
    pub is_pinned: bool,
    pub remark: Option<String>,
    pub nickname: Option<String>,
    pub join_time: String,
}

// 群组列表项
#[derive(Serialize, Deserialize)]
pub struct GroupListItem {
    pub gid: String,
    pub group_name: String,
    pub avatar: String,
    pub bio: String,
}

// 群组列表响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupListResponse {
    pub groups: Vec<GroupListItem>,
    pub total: i64,
}

// 发送群聊申请响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupRequestResponse {
    pub success: bool,
    pub req_id: String,
    pub gid: String,
    pub sender_uid: String,
    pub apply_text: String,
    pub create_time: String,
    pub status: String,
}

// 群聊申请列表项
#[derive(Serialize, Deserialize)]
pub struct GroupRequestItem {
    pub req_id: String,
    pub gid: String,
    pub sender_uid: String,
    pub apply_text: Option<String>,
    pub create_time: String,
    pub status: String,
}

// 获取群聊申请列表响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupRequestListResponse {
    pub requests: Vec<GroupRequestItem>,
    pub total: i64,
}

// 处理群聊申请响应模型（仅返回状态码，不返回具体内容）
pub type GroupRespondResponse = ();

// 退出群聊响应模型
#[derive(Serialize, Deserialize)]
pub struct LeaveGroupResponse {
    pub success: bool,
    pub message: String,
}

// 踢出群成员响应模型
#[derive(Serialize, Deserialize)]
pub struct KickMemberResponse {
    pub message: String,
}

// 解散群聊响应模型
#[derive(Serialize, Deserialize)]
pub struct DisbandGroupResponse {
    pub success: bool,
    pub message: String,
}

// 群成员设置响应模型
#[derive(Serialize, Deserialize)]
pub struct MemberSettingResponse {
    pub success: bool,
    pub message: String,
}

// 修改群聊设置响应模型
#[derive(Serialize, Deserialize)]
pub struct SettingGourpResponse {
    pub message: String,
}

// 群公告项模型
#[derive(Serialize, Deserialize)]
pub struct AnnouncementItem {
    pub msg_id: String,                 // 消息ID
    pub content: String,                // 公告内容
    pub sender_uid: String,             // 发送者UID
    pub send_time: String,              // 发送时间戳
    pub mentioned_uids: Vec<String>,    // 提及的用户ID列表
    pub quote_msg_id: String,           // 引用消息ID
}

// 获取群公告响应模型
#[derive(Serialize, Deserialize)]
pub struct GetAnnouncementsResponse {
    pub announcements: Vec<AnnouncementItem>, // 公告列表
    pub total: i32,             // 总数
}

// 群成员项模型
#[derive(Serialize, Deserialize)]
pub struct MemberItem {
    pub role: String,        // 角色（admin, owner, member）
    pub uid: String,         // 用户ID
    pub username: String,    // 用户名
    pub avatar: String,      // 头像URL
    pub nickname: String,    // 群昵称
}

// 获取群成员列表响应模型
#[derive(Serialize, Deserialize)]
pub struct GetMembersResponse {
    pub members: Vec<MemberItem>,  // 成员列表
    pub total: i32,               // 总数
}

// 转让群主响应模型
#[derive(Serialize, Deserialize)]
pub struct TransferOwnershipResponse {
    pub message: String,       // 操作结果消息
}

// 设置管理员响应模型
#[derive(Serialize, Deserialize)]
pub struct SettingAdminResponse {
    // 可以添加响应字段，如果需要的话
    // 目前为空响应体
}

// 获取禁言状态响应模型
#[derive(Serialize, Deserialize)]
pub struct GetBanStatusResponse {
    pub is_banned: bool,       // 是否被禁言
    pub remain: String,        // 剩余时间戳（如果未禁言则为空）
}

// 禁言成员响应模型
#[derive(Serialize, Deserialize)]
pub struct BanningMemberResponse {
    // 空响应体，成功时返回200 OK
}

// 解除禁言响应模型
#[derive(Serialize, Deserialize)]
pub struct RemoveMuteResponse {
    // 空响应体，成功时返回200 OK
}

// 聊天类型枚举
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ChatType {
    Private,
    Group,
}

// 聊天项模型
#[derive(Serialize, Deserialize)]
pub struct ChatItem {
    pub id: String,
    pub is_pinned: bool,
    #[serde(rename = "type")]
    pub chat_type: ChatType, // "private" or "group"
    pub latest_message: String,
    pub updated_at: String, // 时间戳字符串
    pub unread_messages: i32,
    pub avatar: String,
    pub remark: String, // 备注/名字
}

// 聊天列表响应模型
#[derive(Serialize, Deserialize)]
pub struct ChatListResponse {
    pub chats: Vec<ChatItem>,
}

// 私聊响应模型
#[derive(Serialize, Deserialize)]
pub struct PrivateChatResponse {
    pub id: String,
    pub is_pinned: bool,
    #[serde(rename = "type")]
    pub chat_type: String, // "private"
    pub latest_message: String,
    pub updated_at: String, // ISO 8601 格式的时间字符串
    pub avatar: String,
    pub remark: String, // 备注名，如果没有备注则显示用户名
}

// 群聊响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupChatResponse {
    pub id: String,                    // 群组ID
    pub is_pinned: bool,               // 是否置顶
    #[serde(rename = "type")]
    pub chat_type: String,             // 聊天类型，固定为"group"
    pub latest_message: String,        // 最新消息内容
    pub updated_at: String,            // 最新消息时间戳
    pub avatar: String,                // 群组头像URL
    pub remark: String,                // 群组名称或用户自定义备注
}



// 私聊历史响应模型
#[derive(Serialize, Deserialize)]
pub struct PrivateHistoryResponse {

}

// 群聊历史响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupHistoryResponse {

}

// 标记已读响应模型
#[derive(Serialize, Deserialize)]
pub struct ReadResponse {

}

// 上传文件响应模型
#[derive(Serialize, Deserialize)]
pub struct UploadFileResponse {

}

// 预览文件响应模型
#[derive(Serialize, Deserialize)]
pub struct PreviewFileResponse {

}

// 下载文件响应模型
#[derive(Serialize, Deserialize)]
pub struct DownloadFileResponse {

}

// 删除文件响应模型
#[derive(Serialize, Deserialize)]
pub struct DeleteFileResponse {

}

// 好友在线状态响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendsOnlineResponse {

}

// 群组在线状态响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupOnlineResponse {

}
