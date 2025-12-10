use chrono::NaiveDateTime;
use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::errors::AppResult;
use crate::models::repository::GroupMessageRepository;
use crate::models::entities::GroupMessage;
use crate::models::entities::GroupMsgType;

#[async_trait]
impl GroupMessageRepository for MySqlPool {
    //-------------------------群聊消息管理--------------------------------
    // 保存群聊消息
    async fn save_message(&self, msg: GroupMessage) -> AppResult<()>{

        let mut tx=self.begin().await?;

        sqlx::query!(
            "INSERT INTO group_message(
                msg_id,
                gid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                type,
                mentioned_uids,
                quote_msg_id,
                is_announcement)
                VALUES (?,?,?,?,?,?,?,?,?,?)
            ON DUPLICATE KEY UPDATE
                content = VALUES(content),
                sender_uid = VALUES(sender_uid),
                send_time = VALUES(send_time),
                is_revoked = VALUES(is_revoked),
                type = VALUES(type),
                mentioned_uids = VALUES(mentioned_uids),
                quote_msg_id = VALUES(quote_msg_id),
                is_announcement = VALUES(is_announcement)",
                msg.msg_id,
                msg.gid,
                msg.content,
                msg.sender_uid,
                msg.send_time,
                msg.is_revoked,
                msg.msg_type,
                msg.mentioned_uids,
                msg.quote_msg_id,
                msg.is_announcement
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    // 按msg_id查找群聊消息
    async fn find_message_by_id(&self, msg_id: &str) -> AppResult<Option<GroupMessage>>{

        let find_message=sqlx::query_as!(
            GroupMessage,
            "SELECT 
                msg_id,
                gid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                type as `msg_type: GroupMsgType`,
                mentioned_uids,
                quote_msg_id,
                is_announcement
            FROM group_message WHERE msg_id = ?",
            msg_id
        ).fetch_optional(self).await?;

        Ok(find_message)

    }
    // 按gid查找群聊消息
    async fn find_messages_by_group(&self, gid: &str) -> AppResult<Vec<GroupMessage>>{

        let find_message=sqlx::query_as!(
            GroupMessage,
            "SELECT 
                msg_id,
                gid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                type as `msg_type: GroupMsgType`,
                mentioned_uids,
                quote_msg_id,
                is_announcement
            FROM group_message WHERE gid = ?",
            gid
        ).fetch_all(self).await?;

        Ok(find_message)
    }
    // 按gid和时间范围查找群聊消息
    async fn find_messages_by_group_and_time_range(&self, gid: &str, start: NaiveDateTime, end: NaiveDateTime) -> AppResult<Vec<GroupMessage>>{

        let find_message_by_time=sqlx::query_as!(
            GroupMessage,
            "SELECT 
                msg_id,
                gid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                type as `msg_type: GroupMsgType`,
                mentioned_uids,
                quote_msg_id,
                is_announcement
            FROM group_message WHERE gid = ? AND send_time BETWEEN ? AND ?
            ORDER BY send_time ASC",
            gid,
            start,
            end
        ).fetch_all(self).await?;

        Ok(find_message_by_time)
        
    }
    // 标记消息为已撤回
    async fn mark_message_as_revoked(&self, msg_id: &str) -> AppResult<()>{

        let mut tx=self.begin().await?;

        sqlx::query!(
            "UPDATE group_message 
                SET is_revoked = true
            WHERE msg_id = ?",
            msg_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    //按gid查找查看群公告
    async fn find_announces_by_group(&self, gid: &str) -> AppResult<Vec<GroupMessage>>{

        let find_message=sqlx::query_as!(
            GroupMessage,
            "SELECT 
                msg_id,
                gid,
                content,
                sender_uid,
                send_time,
                is_revoked,
                type as `msg_type: GroupMsgType`,
                mentioned_uids,
                quote_msg_id,
                is_announcement
            FROM group_message WHERE gid = ? AND is_announcement = true",
            gid
        ).fetch_all(self).await?;

        Ok(find_message)
    }
//-------------------------消息已读状态管理--------------------------------
    // 标记消息为已读
    async fn mark_message_as_read(&self, msg_id: &str, gid: &str, uid: &str) -> AppResult<()>{

        let mut tx=self.begin().await?;

        sqlx::query!(
            "INSERT IGNORE INTO group_message_read(msg_id,gid,uid) 
            VALUES (?,?,?)",
            msg_id,
            gid,
            uid
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())

    }

    // 查找消息的已读用户
    async fn find_read_users_by_message(&self, msg_id: &str) -> AppResult<Vec<String>>{

        let users_by_msg_id=sqlx::query!(
            "SELECT uid FROM group_message_read WHERE msg_id = ?",
            msg_id
        ).fetch_all(self).await?;

        let users:Vec<String>=users_by_msg_id.into_iter().map(|r| r.uid).collect();

        Ok(users)
    }

    // 查找用户未读消息
    async fn find_unread_messages_by_user(&self, gid: &str, uid: &str) -> AppResult<Vec<GroupMessage>>{

        let message=sqlx::query_as!(
            GroupMessage,
        "SELECT 
            gm.msg_id,
            gm.gid,
            gm.content,
            gm.sender_uid,
            gm.send_time,
            gm.is_revoked,
            gm.type as `msg_type: GroupMsgType`,
            gm.mentioned_uids,
            gm.quote_msg_id,
            gm.is_announcement
        FROM group_message gm
        LEFT JOIN group_message_read gmr 
            ON gm.msg_id = gmr.msg_id 
            AND gm.gid = gmr.gid 
            AND gmr.uid = ?
        WHERE gm.gid = ? AND gmr.msg_id IS NULL
        ORDER BY gm.send_time DESC",
            uid,
            gid
        ).fetch_all(self).await?;

        Ok(message)
    }

    // 查找消息已读用户数量
    async fn get_message_read_count(&self, msg_id: &str) -> AppResult<u64>{

        let already=sqlx::query!(
            "SELECT COUNT(*) as count FROM group_message_read WHERE msg_id = ?",
            msg_id
        ).fetch_one(self).await?;

        Ok(already.count as u64)

    }
}
