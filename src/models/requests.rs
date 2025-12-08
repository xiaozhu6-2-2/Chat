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
    pub avator: String,
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

}

// 群组资料请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupProfileRequest {

}

// 群组列表请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupListRequest {

}

// 聊天列表请求模型
#[derive(Deserialize, Serialize)]
pub struct ChatListRequest {

}

// 私聊请求模型
#[derive(Deserialize, Serialize)]
pub struct PrivateChatRequest {

}

// 群聊请求模型
#[derive(Deserialize, Serialize)]
pub struct GroupChatRequest {

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


