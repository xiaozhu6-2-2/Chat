use sqlx::MySqlPool;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::models::errors::AppResult;
use crate::models::repository::PrivateChatRepository;
use crate::models::entities::{PrivateChat, PrivateMessage, PrivateMsgType, EnumConvertible};

#[async_trait]
impl PrivateChatRepository for MySqlPool {
    //-------------------------私聊会话管理-----------------------
    // 保存私聊会话
    async fn save_chat(&self, chat: PrivateChat) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        // 插入或更新私聊会话
        sqlx::query!(
            "INSERT INTO private_chat
            (pid, uid1, uid2, create_time, is_pinned_by_uid1, is_pinned_by_uid2)
            VALUES (?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
            create_time = VALUES(create_time),
            is_pinned_by_uid1 = VALUES(is_pinned_by_uid1),
            is_pinned_by_uid2 = VALUES(is_pinned_by_uid2)",
            chat.pid,
            chat.uid1,
            chat.uid2,
            chat.create_time,
            chat.is_pinned_by_uid1,
            chat.is_pinned_by_uid2,
        ).execute(&mut *tx).await?;

        // 提交事务
        tx.commit().await?;

        Ok(())
    }

    // 按pid查找私聊会话
    async fn find_chat_by_pid(&self, pid: &str) -> AppResult<Option<PrivateChat>> {
        let chat = sqlx::query_as!(
            PrivateChat,
            "SELECT
                pid, uid1, uid2, create_time, is_pinned_by_uid1, is_pinned_by_uid2, do_not_disturb_uid1, do_not_disturb_uid2
            FROM private_chat WHERE pid = ?",
            pid
        ).fetch_optional(self).await?;

        Ok(chat)
    }

    // 查找两个用户的私聊会话
    async fn find_chat_by_users(&self, uid1: &str, uid2: &str) -> AppResult<Option<PrivateChat>> {
        // 确保较小的uid在前，较大的uid在后（保证数据一致性）
        let (smaller_uid, larger_uid) = if uid1 < uid2 {
            (uid1, uid2)
        } else {
            (uid2, uid1)
        };

        let chat = sqlx::query_as!(
            PrivateChat,
            "SELECT
                pid, uid1, uid2, create_time, is_pinned_by_uid1, is_pinned_by_uid2, do_not_disturb_uid1, do_not_disturb_uid2
            FROM private_chat
            WHERE uid1 = ? AND uid2 = ?",
            smaller_uid,
            larger_uid
        ).fetch_optional(self).await?;

        Ok(chat)
    }

    // 查找用户的私聊会话
    async fn find_chats_by_user(&self, uid: &str) -> AppResult<Vec<PrivateChat>> {
        let chats = sqlx::query_as!(
            PrivateChat,
            "SELECT
                pid, uid1, uid2, create_time, is_pinned_by_uid1, is_pinned_by_uid2, do_not_disturb_uid1, do_not_disturb_uid2
            FROM private_chat
            WHERE uid1 = ? OR uid2 = ?
            ORDER BY create_time DESC",
            uid,
            uid
        ).fetch_all(self).await?;

        Ok(chats)
    }

    // 更新用户置顶状态
    async fn update_pin_status(&self, pid: &str, uid: &str, is_pinned: bool) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        // 根据uid确定更新哪个置顶字段
        let result = sqlx::query!(
            "SELECT uid1, uid2 FROM private_chat WHERE pid = ?",
            pid
        ).fetch_one(&mut *tx).await;

        let pinned_value = if is_pinned { 1 } else { 0 };

        match result {
            Ok(chat_info) => {
                if uid == chat_info.uid1 {
                    sqlx::query!(
                        "UPDATE private_chat
                        SET is_pinned_by_uid1 = ?
                        WHERE pid = ?",
                        pinned_value,
                        pid
                    ).execute(&mut *tx).await?;
                } else if uid == chat_info.uid2 {
                    sqlx::query!(
                        "UPDATE private_chat
                        SET is_pinned_by_uid2 = ?
                        WHERE pid = ?",
                        pinned_value,
                        pid
                    ).execute(&mut *tx).await?;
                }
            }
            Err(_) => {
                // 如果找不到会话，不做操作
            }
        }

        tx.commit().await?;

        Ok(())
    }

    //-------------------------私聊消息管理-----------------------
    // 保存私聊消息
    async fn save_message(&self, msg: PrivateMessage) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO private_message(
                msg_id,
                pid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                is_read,
                type)
                VALUES (?,?,?,?,?,?,?,?)
            ON DUPLICATE KEY UPDATE
                content = VALUES(content),
                sender_uid = VALUES(sender_uid),
                send_time = VALUES(send_time),
                is_revoked = VALUES(is_revoked),
                is_read = VALUES(is_read),
                type = VALUES(type)",
                msg.msg_id,
                msg.pid,
                msg.content,
                msg.sender_uid,
                msg.send_time,
                msg.is_revoked,
                msg.is_read,
                msg.mes_type.to_enum_string()
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    // 按msg_id查找私聊消息
    async fn find_message_by_id(&self, msg_id: &str) -> AppResult<Option<PrivateMessage>> {
        let message = sqlx::query_as!(
            PrivateMessage,
            "SELECT
                msg_id,
                pid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                is_read,
                type as `mes_type: PrivateMsgType`
            FROM private_message WHERE msg_id = ?",
            msg_id
        ).fetch_optional(self).await?;

        Ok(message)
    }

    // 按pid查找私聊消息
    async fn find_messages_by_chat(&self, pid: &str) -> AppResult<Vec<PrivateMessage>> {
        let messages = sqlx::query_as!(
            PrivateMessage,
            "SELECT
                msg_id,
                pid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                is_read,
                type as `mes_type: PrivateMsgType`
            FROM private_message
            WHERE pid = ?
            ORDER BY send_time ASC",
            pid
        ).fetch_all(self).await?;

        Ok(messages)
    }

    // 标记消息为已读
    async fn mark_message_as_read(&self, msg_id: &str) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "UPDATE private_message
                SET is_read = 1
            WHERE msg_id = ?",
            msg_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    // 标记消息为撤回
    async fn mark_message_as_revoked(&self, msg_id: &str) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "UPDATE private_message
                SET is_revoked = 1
            WHERE msg_id = ?",
            msg_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    // 查找未读消息
    async fn find_unread_message_by_chat(&self, pid: &str, uid: &str) -> AppResult<Vec<PrivateMessage>> {
        // 首先获取会话信息以确定哪个uid是接收方
        let chat_info = sqlx::query!(
            "SELECT uid1, uid2 FROM private_chat WHERE pid = ?",
            pid
        ).fetch_optional(self).await?;

        match chat_info {
            Some(_chat) => {
                let messages = sqlx::query_as!(
                    PrivateMessage,
                    "SELECT
                        msg_id,
                        pid,
                        content,
                        sender_uid,
                        send_time,
                        is_revoked,
                        is_read,
                        type as `mes_type: PrivateMsgType`
                    FROM private_message
                    WHERE pid = ?
                    AND sender_uid != ?
                    AND (is_read IS NULL OR is_read = 0)
                    ORDER BY send_time DESC",
                    pid,
                    uid
                ).fetch_all(self).await?;

                Ok(messages)
            }
            None => Ok(vec![])
        }
    }

    // 获取未读消息数量
    async fn get_unread_message_count_by_chat(&self, pid: &str, uid: &str) -> AppResult<i32> {
        // 首先获取会话信息以确定哪个uid是接收方
        let chat_info = sqlx::query!(
            "SELECT uid1, uid2 FROM private_chat WHERE pid = ?",
            pid
        ).fetch_optional(self).await?;

        match chat_info {
            Some(_chat) => {
                let count = sqlx::query!(
                    "SELECT COUNT(*) as count
                    FROM private_message
                    WHERE pid = ?
                    AND sender_uid != ?
                    AND (is_read IS NULL OR is_read = 0)",
                    pid,
                    uid
                ).fetch_one(self).await?;

                Ok(count.count as i32)
            }
            None => Ok(0)
        }
    }

    // 查找会话的最新消息
    async fn find_latest_message_by_chat(&self, pid: &str) -> AppResult<Option<PrivateMessage>> {
        let message = sqlx::query_as!(
            PrivateMessage,
            "SELECT
                msg_id,
                pid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                is_read,
                type as `mes_type: PrivateMsgType`
            FROM private_message
            WHERE pid = ?
            ORDER BY send_time DESC
            LIMIT 1",
            pid
        ).fetch_optional(self).await?;

        Ok(message)
    }

    // 获取私聊会话的消息总数
    async fn get_message_count_by_chat(&self, pid: &str) -> AppResult<i64> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count
            FROM private_message
            WHERE pid = ?",
            pid
        ).fetch_one(self).await?;

        Ok(count.count as i64)
    }

    // 批量标记私聊消息为已读
    async fn mark_messages_as_read_by_chat_and_time(&self, pid: &str, uid: &str, timestamp: DateTime<Utc>) -> AppResult<u64> {
        let mut tx = self.begin().await?;

        let result = sqlx::query!(
            "UPDATE private_message
                SET is_read = 1
            WHERE pid = ?
                AND sender_uid != ?
                AND send_time <= ?
                AND (is_read IS NULL OR is_read = 0)",
            pid,
            uid,
            timestamp
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(result.rows_affected())
    }
}