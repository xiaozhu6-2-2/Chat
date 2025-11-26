use log::{error, info};

use crate::{AppState, models::repository::OnlineRepository, repository::OnlineRepository::OnlineManager};


#[derive(Clone)]
pub struct ConnectionResourcesManager {
    uid: String,// 用于监听任务管理
    account: String,// 用于tx、在线状态管理
    is_tx: bool,
    is_online: bool,
    is_listener: bool,
    group_ids: Vec<String>, // 存储用户所在的群聊ID，用于清理在线状态
}

impl ConnectionResourcesManager {
    pub fn new(uid: &String, account: &String) -> Self {
        Self {
            uid: uid.clone(),
            account: account.clone(),
            is_tx: false,
            is_online: false,
            is_listener: false,
            group_ids: Vec::new(),
        }
    }

    pub fn set_group_ids(&mut self, group_ids: Vec<String>) {
        self.group_ids = group_ids;
    }

    pub fn add_group_id(&mut self, group_id: String) {
        if !self.group_ids.contains(&group_id) {
            self.group_ids.push(group_id);
        }
    }

    pub fn set_tx(&mut self, flag: bool) {
        self.is_tx = flag;       
    }

    pub fn set_online(&mut self, flag: bool) {
        self.is_online = flag;
    }

    pub fn set_listener(&mut self, flag: bool) {
        self.is_listener = flag;
    }

    pub async fn cleanup_resources(&self, state: AppState) {
        if self.is_tx {
            state.connection_pool.remove(&self.account);
            info!("清除 {} 的连接", self.account);
        }

        if self.is_online {
            if let Err(e) = OnlineManager::user_offline(&state.redis_pool, self.account.clone(), &self.group_ids).await {
                log::error!("清理用户在线状态失败: {}", e);
            }
        }

        if self.is_listener {
            if let Err(e) = state.group_task_manager.remove_all_user_tasks(&self.uid).await {
                error!("清理监听任务失败{}", e);
            }
        }
        
        info!("{}连接的资源清理完毕", self.account);
    }
}