use chrono::NaiveDateTime;
// src/models.rs
// 库模块导入
use serde::{Deserialize, Serialize};

// 分离模块导入

// 注册请求结构体
#[derive(Deserialize, Serialize, Clone)]
pub struct RegisterRequest {
    pub account: String,
    pub password: String,
    pub username: String,
    pub gender: String,
    pub region: String,
    pub bio: String,
    pub avatar: String,
}

// 登录请求模型
#[derive(Deserialize, Serialize)]
pub struct LoginRequest {
    pub account: String,
    pub password: String,
}

// 获取用户信息请求模型
#[derive(Deserialize, Serialize)]
pub struct UserInfoRequest {

}

// 用户信息更新请求模型
#[derive(Deserialize, Serialize)]
pub struct UserInfoUpdateRequest {
    pub username: String,
    pub gender: Option<String>,        
    pub region: Option<String>,        
    pub email: Option<String>,         
    pub avatar: Option<String>,        
    pub bio: Option<String>,  
}

// 获取用户资料请求模型
#[derive(Deserialize, Serialize)]
pub struct FetchProfileRequest {
    pub uid: String,
}

// 搜索用户请求模型
#[derive(Deserialize, Serialize)]
pub struct SearchUserRequest {
    pub query: String,// 可以是用户名/uid/account
    pub limit: i64,// 每页的条目数
    pub offset: i64,// 第几页
}

// 好友资料请求模型
#[derive(Deserialize, Serialize)]
pub struct FriendProfileRequest {
    pub uid: String,// 好友的uid
    pub fid: String,// 好友关系的id
}

// 好友请求请求模型
#[derive(Deserialize, Serialize)]
pub struct FriendRequestRequest {
    pub receiver_id: String,// 接收者id
    pub message: String,// 申请消息
    pub create_time: NaiveDateTime,// 好友请求创建时间
}

// 回复好友请求请求模型
#[derive(Deserialize, Serialize)]
pub struct RespondFriendRequestRequest {
    pub req_id: String,      // 好友请求ID
    pub action: String,      // 操作类型: "accept" 或 "reject"
    pub handle_time: String, // 处理时间戳
}

// 好友请求列表请求模型
#[derive(Deserialize, Serialize)]
pub struct FriendRequestListRequest {

}

// 删除好友请求模型
#[derive(Deserialize, Serialize)]
pub struct RemoveFriendRequest {
    pub fid: String,
}

// 更新好友备注请求模型
#[derive(Deserialize, Serialize)]
pub struct UpdateFriendRemarkBlacklistGroupByRequest {
    pub fid: String,
    pub remark: Option<String>,
    pub is_blacklisted: bool,
    pub group_by: Option<String>,
}

// 更新好友黑名单请求模型
#[derive(Deserialize, Serialize)]
pub struct UpdateFriendBlacklistRequest {

}

// 搜索群组请求模型
#[derive(Deserialize, Serialize)]
pub struct SearchGroupRequest {
    pub query: String,
    pub limit: i64,
    pub offset: i64,
}

// 群组名片请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupCardRequest {
    pub gid: String,
}

// 创建群组请求模型
#[derive(Deserialize, Serialize)]
pub struct CreateGroupRequest {
    pub manager_uid: String,
    pub group_name: String,          // 必填
    pub avatar: Option<String>,      // 可选
    pub created_at: String,
    pub group_intro: Option<String>, // 可选
}

//群组信息请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupProfileRequest {
    pub gid: String,
}

// 群组列表请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupListRequest;

//发送加入群聊申请请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupRequestRequest {
    pub gid: String,
    pub uid: String,
    pub apply_text: String,
    pub create_time: String,
}

// 获取群聊申请列表请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupRequestListRequest {
    pub gid: String,
    pub uid: String,
}

// 处理群聊申请请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupRespondRequest {
    pub req_id: String,
    pub approver_uid: String,
    pub action: String,  // "accept" 或 "reject"
    pub handle_time: String,
}

// 退出群聊请求模型
#[derive(Deserialize, Serialize)]
pub struct LeaveGroupRequest {
    pub gid: String,
    pub uid: String,
}

// 踢出群成员请求模型
#[derive(Deserialize, Serialize)]
pub struct KickMemberRequest {
    pub gid: String,          // 群组ID
    pub uid: String,          // 被踢出的群员ID
    pub approver_uid: String,  // 执行踢人的管理员ID
}

// 解散群聊请求模型
#[derive(Deserialize, Serialize)]
pub struct DisbandGroupRequest {
    pub gid: String,  // 群组ID
}

// 群成员设置请求模型
#[derive(Deserialize, Serialize)]
pub struct MemberSettingRequest {
    pub gid: String,           // 群组ID
    pub do_not_disturb: bool,  // 是否免打扰
    pub is_pinned: bool,       // 是否置顶
    pub remark: String,        // 备注
    pub nickname: String,      // 群昵称
}

// 修改群聊设置请求模型
#[derive(Deserialize, Serialize)]
pub struct SettingGroupRequest {
    pub gid: String,            // 群组ID
    pub group_name: String,     // 群名称
    pub group_avater: String,   // 群头像URL
    pub group_intro: String,    // 群简介
    pub uid: String,            // 修改者uid
}

// 获取群公告请求模型
#[derive(Deserialize, Serialize)]
pub struct GetAnnouncementsRequest {
    pub gid: String,         // 群组ID
}

// 获取群成员列表请求模型
#[derive(Deserialize, Serialize)]
pub struct GetMembersRequest {
    pub gid: String,         // 群组ID
}

// 转让群主请求模型
#[derive(Deserialize, Serialize)]
pub struct TransferOwnershipRequest {
    pub gid: String,         // 群组ID
    pub manager_uid: String,  // 转让者uid（当前群主）
    pub uid: String,         // 被转让者uid
}

// 设置管理员请求模型
#[derive(Deserialize, Serialize)]
pub struct SettingAdminRequest {
    pub gid: String,         // 群组ID
    pub uid: String,         // 要设置为管理员的用户uid
}

// 获取禁言状态请求模型
#[derive(Deserialize, Serialize)]
pub struct GetBanStatusRequest {
    pub gid: String,         // 群组ID
}

// 禁言成员请求模型
#[derive(Deserialize, Serialize)]
pub struct BanningMemberRequest {
    pub gid: String,         // 群组ID
    pub uid: String,         // 被禁言的群成员ID
    pub time: String,        // 禁言时长（秒），-1表示永久禁言
}

// 解除禁言请求模型
#[derive(Deserialize, Serialize)]
pub struct RemoveMuteRequest {
    pub gid: String,         // 群组ID
    pub uid: String,         // 被解除禁言的群成员ID
}

// 聊天列表请求模型
#[derive(Deserialize, Serialize)]
pub struct ChatListRequest {

}

// 私聊请求模型
#[derive(Deserialize, Serialize)]
pub struct PrivateChatRequest {
    pub fid: String,// 好友关系id
}

// 群聊请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupChatRequest {
    pub gid: String,// 群聊id
}

// 私聊历史请求模型
#[derive(Deserialize, Serialize)]
pub struct PrivateHistoryRequest {

}

// 群聊历史请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupHistoryRequest {

}

// 标记已读请求模型
#[derive(Deserialize, Serialize)]
pub struct ReadRequest {

}

// 上传文件请求模型
#[derive(Deserialize, Serialize)]
pub struct UploadFileRequest {

}

// 预览文件请求模型
#[derive(Deserialize, Serialize)]
pub struct PreviewFileRequest {

}

// 下载文件请求模型
#[derive(Deserialize, Serialize)]
pub struct DownloadFileRequest {

}

// 删除文件请求模型
#[derive(Deserialize, Serialize)]
pub struct DeleteFileRequest {

}

// 好友在线状态请求模型
#[derive(Deserialize, Serialize)]
pub struct FriendsOnlineRequest {

}

// 群组在线状态请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupOnlineRequest {

}


