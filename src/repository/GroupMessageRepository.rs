use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::errors::AppResult;
use crate::models::repository::GroupMessageRepository;
use crate::models::entities::{GroupMessage, GroupMsgType, EnumConvertible};

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
                is_revoked,
                type,
                mentioned_uids,
                quote_msg_id,
                is_announcement)
                VALUES (?,?,?,?,?,?,?,?,?)
            ON DUPLICATE KEY UPDATE
                content = VALUES(content),
                sender_uid = VALUES(sender_uid),
                is_revoked = VALUES(is_revoked),
                type = VALUES(type),
                mentioned_uids = VALUES(mentioned_uids),
                quote_msg_id = VALUES(quote_msg_id),
                is_announcement = VALUES(is_announcement)",
                msg.msg_id,
                msg.gid,
                msg.content,
                msg.sender_uid,
                msg.is_revoked,
                msg.msg_type.to_enum_string(),
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
            FROM group_message WHERE gid = ?
            ORDER BY send_time DESC",
            gid
        ).fetch_all(self).await?;

        Ok(find_message)
    }

    // 获取群聊消息总数
    async fn get_message_count_by_group(&self, gid: &str) -> AppResult<i64>{
        let result = sqlx::query!(
            "SELECT COUNT(*) as count FROM group_message WHERE gid = ?",
            gid
        )
        .fetch_one(self)
        .await?;

        Ok(result.count as i64)
    }

    // 按gid分页查找群聊消息
    async fn find_messages_by_group_with_pagination(&self, gid: &str, limit: i64, offset: i64) -> AppResult<Vec<GroupMessage>>{
        let messages = sqlx::query_as!(
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
            FROM group_message
            WHERE gid = ?
            ORDER BY send_time DESC
            LIMIT ? OFFSET ?",
            gid,
            limit,
            offset * limit  // 计算偏移量
        ).fetch_all(self).await?;

        Ok(messages)
    }

    // 按gid和时间范围查找群聊消息
    async fn find_messages_by_group_and_time_range(&self, gid: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> AppResult<Vec<GroupMessage>>{

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
            ORDER BY send_time DESC",
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

    // 批量获取多个消息的已读数量
    async fn get_message_read_counts(&self, msg_ids: &[String]) -> AppResult<Vec<(String, i64)>> {
        if msg_ids.is_empty() {
            return Ok(Vec::new());
        }

        // 构建占位符字符串 (?, ?, ?, ...)
        let placeholders = msg_ids.iter()
            .map(|_| "?")
            .collect::<Vec<_>>()
            .join(",");

        let query_str = format!(
            "SELECT msg_id, COUNT(*) as count
             FROM group_message_read
             WHERE msg_id IN ({})
             GROUP BY msg_id",
            placeholders
        );

        // 将 msg_ids 转换为可以绑定到查询的参数
        let mut query = sqlx::query_as::<_, (String, i64)>(&query_str);
        for msg_id in msg_ids {
            query = query.bind(msg_id);
        }

        let results = query.fetch_all(self).await?;
        Ok(results)
    }

    // 获取用户未读消息数量
    async fn get_unread_message_count_by_group(&self, gid: &str, uid: &str) -> AppResult<i32> {
        let count = sqlx::query!(
            "SELECT COUNT(*) as count
            FROM group_message gm
            LEFT JOIN group_message_read gmr
                ON gm.msg_id = gmr.msg_id
                AND gm.gid = gmr.gid
                AND gmr.uid = ?
            WHERE gm.gid = ? AND gmr.msg_id IS NULL",
            uid,
            gid
        ).fetch_one(self).await?;

        Ok(count.count as i32)
    }

    // 查找群聊的最新消息
    async fn find_latest_message_by_group(&self, gid: &str) -> AppResult<Option<GroupMessage>> {
        let message = sqlx::query_as!(
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
            FROM group_message
            WHERE gid = ?
            ORDER BY send_time DESC
            LIMIT 1",
            gid
        ).fetch_optional(self).await?;

        Ok(message)
    }

    // 批量标记群聊消息为已读
    async fn mark_messages_as_read_by_group_and_time(&self, gid: &str, uid: &str, timestamp: DateTime<Utc>) -> AppResult<u64> {
        let mut tx = self.begin().await?;

        // 使用 INSERT IGNORE 批量插入已读记录
        let result = sqlx::query!(
            "INSERT IGNORE INTO group_message_read (msg_id, gid, uid)
            SELECT gm.msg_id, gm.gid, ?
            FROM group_message gm
            LEFT JOIN group_message_read gmr
                ON gm.msg_id = gmr.msg_id
                AND gm.gid = gmr.gid
                AND gmr.uid = ?
            WHERE gm.gid = ?
                AND gm.send_time <= ?
                AND gmr.msg_id IS NULL",
            uid,
            uid,
            gid,
            timestamp
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(result.rows_affected())
    }
}
