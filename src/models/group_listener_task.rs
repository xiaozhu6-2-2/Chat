use std::sync::Arc;

use axum::extract::ws::Message;
use dashmap::DashMap;
use tokio::sync::oneshot;
use tokio::{sync::mpsc::UnboundedSender, time::Instant};
use tokio_util::sync::CancellationToken;

use crate::models::{errors::AppResult, others::GroupBroadcastChannel};

// 群聊监听任务的信息
#[derive(Clone)]
pub struct GroupListenerTask {
    pub task_id: String,
    pub gid: String,
    pub uid: String,
    pub cancel_token: CancellationToken,// 取消令牌
    pub created_at: Instant,
}

// 控制命令枚举
#[derive(Debug)]
pub enum TaskCommand {
    AddListener {
        uid: String,
        account: String,
        gid: String,
        tx: UnboundedSender<Message>,// 用于向用户推送信息
        broadcast_pool: Arc<DashMap<String, GroupBroadcastChannel>>,
        response: oneshot::Sender<AppResult<String>>,
    },
    RemoveListener {
        uid: String,
        gid: String,
        response: oneshot::Sender<AppResult<()>>,
    },
    RemoveAllUserTasks {
        uid: String,
        response: oneshot::Sender<AppResult<()>>,
    },
    GetTaskStatus {
        uid: String,
        gid: String,
        response: oneshot::Sender<AppResult<bool>>
    }
}