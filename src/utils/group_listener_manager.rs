use core::task;
use std::{collections::HashMap, sync::Arc};

use dashmap::DashMap;
use log::info;
use tokio::{sync::{RwLock, mpsc, oneshot}, task::JoinHandle};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{utils::trans_logic::group_channel_listen, models::errors::AppError, models::group_listener_task::{GroupListenerTask, TaskCommand}, models::others::GroupBroadcastChannel, AppResult};

// 用户群聊监听任务管理器
#[derive(Clone)]
pub struct UserGroupTaskManager {
    // 任务ID -> 任务信息的映射
    pub active_tasks: Arc<RwLock<HashMap<String, GroupListenerTask>>>,
    // 用户ID -> 任务ID列表的映射
    pub user_tasks: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // 群聊ID -> 监听用户列表的映射
    pub group_listeners: Arc<RwLock<HashMap<String, Vec<String>>>>,
    // 任务命令发送通道
    pub task_command_tx: mpsc::UnboundedSender<TaskCommand>,
}

impl UserGroupTaskManager {
    pub fn new() -> Self {
        let (task_command_tx, task_command_rx) = mpsc::unbounded_channel();

        let manager = Self {
            active_tasks: Arc::new(RwLock::new(HashMap::new())),
            user_tasks: Arc::new(RwLock::new(HashMap::new())),
            group_listeners: Arc::new(RwLock::new(HashMap::new())),
            task_command_tx,
        };

        // 启动任务管理器协程
        let manager_clone = manager.clone();
        tokio::spawn(async move {
            manager_clone.run_task_manager(task_command_rx).await;
        });

        manager
    }
    
    // 公共接口方法
    pub async fn add_listener(
        &self,
        user_id: String,
        account: String,
        group_id: String,
        tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
        broadcast_pool: Arc<DashMap<String, GroupBroadcastChannel>>,
    ) -> AppResult<String> {
        let (response_tx, response_rx) = oneshot::channel();

        self.task_command_tx.send(TaskCommand::AddListener {
            uid: user_id,
            account,
            gid: group_id,
            tx,
            broadcast_pool,
            response: response_tx,
        }).map_err(|_| AppError::TaskManagerError("Command channel closed".to_string()))?;

        response_rx.await.map_err(|_| AppError::TaskManagerError("Response channel closed".to_string()))?
    }

    pub async fn remove_listener(&self, user_id: &str, group_id: &str) -> AppResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.task_command_tx.send(TaskCommand::RemoveListener {
            uid: user_id.to_string(),
            gid: group_id.to_string(),
            response: response_tx,
        }).map_err(|_| AppError::TaskManagerError("Command channel closed".to_string()))?;

        response_rx.await.map_err(|_| AppError::TaskManagerError("Response channel closed".to_string()))?
    }

    pub async fn remove_all_user_tasks(&self, user_id: &str) -> AppResult<()> {
        let (response_tx, response_rx) = oneshot::channel();

        self.task_command_tx.send(TaskCommand::RemoveAllUserTasks {
            uid: user_id.to_string(),
            response: response_tx,
        }).map_err(|_| AppError::TaskManagerError("Command channel closed".to_string()))?;

        response_rx.await.map_err(|_| AppError::TaskManagerError("Response channel closed".to_string()))?
    }

    pub async fn get_task_status(&self, user_id: &str, group_id: &str) -> AppResult<bool> {
        let (response_tx, response_rx) = oneshot::channel();

        self.task_command_tx.send(TaskCommand::GetTaskStatus {
            uid: user_id.to_string(),
            gid: group_id.to_string(),
            response: response_tx,
        }).map_err(|_| AppError::TaskManagerError("Command channel closed".to_string()))?;

        response_rx.await.map_err(|_| AppError::TaskManagerError("Response channel closed".to_string()))?
    }

    // 内部实现
    // 任务管理器协程
    async fn run_task_manager(&self, mut command_rx: mpsc::UnboundedReceiver<TaskCommand>) {
        loop {
            match command_rx.recv().await {
                Some(command) => {
                    match command {
                        TaskCommand::AddListener { uid, account,  gid, tx, broadcast_pool, response } => {
                            let result = self.add_listener_internal(uid, account, gid, tx, broadcast_pool).await;
                            let _ = response.send(result);
                        },
                        TaskCommand::RemoveListener { uid, gid, response } => {
                            let result = self.remove_listener_internal(uid, gid).await;
                            let _ = response.send(result);
                        },
                        TaskCommand::RemoveAllUserTasks { uid, response } => {
                            let result = self.remove_all_user_tasks_internal(uid).await;
                            let _ = response.send(result);
                        },
                        TaskCommand::GetTaskStatus { uid, gid, response } => {
                            let result = self.get_task_status_internal(uid, gid).await;
                            let _ = response.send(result);
                        },
                    }
                },
                None => {
                    // 通道关闭，记录日志并退出循环
                    log::error!("任务管理器命令通道已关闭，任务管理器协程退出");
                    break;
                }
            }
        }
    }

    // 添加监听器
    async fn add_listener_internal(
        &self,
        user_id: String,
        account: String,
        group_id: String,
        tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
        broadcast_pool: Arc<DashMap<String, GroupBroadcastChannel>>,
    ) -> AppResult<String> {
        let task_id = Uuid::new_v4().to_string();
        let cancel_token = CancellationToken::new();
        // debug
        info!("添加任务{}监听群聊{}", task_id, group_id);
        // 创建并启动监听任务（调用原来的群聊监听逻辑）
        let task_handle = self.create_listener_task(
            account.clone(),
            group_id.clone(),
            tx,
            broadcast_pool,
            cancel_token.clone(),
        ).await?;

        // 创建任务信息
        let task = GroupListenerTask {
            task_id: task_id.clone(),
            gid: group_id.clone(),
            uid: user_id.clone(),
            cancel_token: cancel_token.clone(),
            created_at: tokio::time::Instant::now(),
        };

        // 更新映射关系
        {
            let mut active_tasks = self.active_tasks.write().await;
            active_tasks.insert(task_id.clone(), task.clone());
        }

        {
            let mut user_tasks = self.user_tasks.write().await;
            user_tasks.entry(user_id.clone()).or_insert_with(Vec::new).push(task_id.clone());
        }

        {
            let mut group_listeners = self.group_listeners.write().await;
            group_listeners.entry(group_id.clone()).or_insert_with(Vec::new).push(user_id.clone());
        }

        // 任务完成时自动清理
        let task_id_for_cleanup = task_id.clone();
        let uid_for_cleanup = user_id.clone();
        let gid_for_cleanup = group_id.clone();
        let active_tasks_for_cleanup = self.active_tasks.clone();
        let user_tasks_for_cleanup = self.user_tasks.clone();
        let group_listeners_for_cleanup = self.group_listeners.clone();

        // 监控任务句柄，完成时清理映射
        tokio::spawn(async move {
            let _ = task_handle.await;
            // 任务完成后自动清理
            Self::cleanup_task(
                &task_id_for_cleanup,
                &uid_for_cleanup,
                &gid_for_cleanup,
                &active_tasks_for_cleanup,
                &user_tasks_for_cleanup,
                &group_listeners_for_cleanup,
            ).await;
        });
        
        // debug
        info!("成功添加{}任务监听群聊{}", task_id, group_id);

        Ok(task_id)
    }

    // 移除监听器
    async fn remove_listener_internal(&self, user_id: String, group_id: String) -> AppResult<()> {
        // 查找对应的任务并触发取消
        let active_tasks = self.active_tasks.read().await;
        // let mut tasks_cancelled = 0;

        // 如果有的话就取消，没有的话就不取消
        for task in active_tasks.values() {
            if task.uid == user_id && task.gid == group_id {
                task.cancel_token.cancel();
                // tasks_cancelled += 1;
            }
        }

        // if tasks_cancelled == 0 {
        //     return Err(AppError::TaskManagerError(format!("用户{}没有群聊监听任务", user_id)));
        // }

        Ok(())
    }

    // 移除用户所有任务
    async fn remove_all_user_tasks_internal(&self, user_id: String) -> AppResult<()> {
        // 查找对应的任务并触发取消
        let active_tasks = self.active_tasks.read().await;
        let mut tasks_cancelled = 0;

        for task in active_tasks.values() {
            if task.uid == user_id {
                task.cancel_token.cancel();
                tasks_cancelled += 1;
            }
        }

        if tasks_cancelled == 0 {
            return Err(AppError::TaskManagerError("No tasks found for user".to_string()));
        }

        Ok(())
    }

    // 获取任务状态
    async fn get_task_status_internal(&self, user_id: String, group_id: String) -> AppResult<bool> {
        let active_tasks = self.active_tasks.read().await;
        for task in active_tasks.values() {
            if task.uid == user_id && task.gid == group_id {
                return Ok(true);
            }
        }
        Ok(false)
    }

     // 创建监听任务
    async fn create_listener_task(
        &self,
        account: String,
        group_id: String,
        tx: mpsc::UnboundedSender<axum::extract::ws::Message>,
        broadcast_pool: Arc<DashMap<String, GroupBroadcastChannel>>,
        cancel_token: CancellationToken,
    ) -> AppResult<JoinHandle<()>> {
        let task_handle = tokio::spawn(async move {
            // 这里调用原有的群聊监听逻辑
            group_channel_listen(
                group_id,
                account,
                tx,
                broadcast_pool,
                cancel_token,
            ).await;
        });

        Ok(task_handle)
    }
    
     // 清理任务
    async fn cleanup_task(
        task_id: &str,
        user_id: &str,
        group_id: &str,
        active_tasks: &Arc<RwLock<HashMap<String, GroupListenerTask>>>,
        user_tasks: &Arc<RwLock<HashMap<String, Vec<String>>>>,
        group_listeners: &Arc<RwLock<HashMap<String, Vec<String>>>>,
    ) {
        // 从活跃任务中移除
        active_tasks.write().await.remove(task_id);

        // 从用户任务列表中移除
        let mut user_tasks_map = user_tasks.write().await;
        if let Some(task_list) = user_tasks_map.get_mut(user_id) {
            // 保留未被指定删除的task
            task_list.retain(|id| id != task_id);
            // 清理空用户列表
            if task_list.is_empty() {
                user_tasks_map.remove(user_id);
            }
        }

        // 从群聊监听者列表中移除
        let mut group_listeners_map = group_listeners.write().await;
        if let Some(listener_list) = group_listeners_map.get_mut(group_id) {
            listener_list.retain(|uid| uid != user_id);
            if listener_list.is_empty() {
                group_listeners_map.remove(group_id);
            }
        }
    }
}
