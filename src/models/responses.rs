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

}

// 好友列表响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendListResponse {

}

// 好友请求响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendRequestResponse {

}

// 回复好友请求响应模型
#[derive(Serialize, Deserialize)]
pub struct RespondFriendRequestResponse {

}

// 好友请求列表响应模型
#[derive(Serialize, Deserialize)]
pub struct FriendRequestListResponse {

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

// 搜索群组响应模型
#[derive(Serialize, Deserialize)]
pub struct SearchGroupResponse {

}

// 群组资料响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupProfileResponse {

}

// 群组列表响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupListResponse {

}

// 聊天列表响应模型
#[derive(Serialize, Deserialize)]
pub struct ChatListResponse {

}

// 私聊响应模型
#[derive(Serialize, Deserialize)]
pub struct PrivateChatResponse {

}

// 群聊响应模型
#[derive(Serialize, Deserialize)]
pub struct GroupChatResponse {

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
