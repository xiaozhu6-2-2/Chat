use chrono::{DateTime, Utc};
use sqlx::MySqlPool;
use async_trait::async_trait;

use crate::models::errors::AppResult;
use crate::models::repository::FriendshipRepository;
use crate::models::entities::{Friends, FriendRequest};
use crate::models::entities::ReqStatus;

#[async_trait]
impl FriendshipRepository for MySqlPool {

    //-------------------------好友关系管理----------------------------
    // 保存好友关系(若数据库有该关系，则修改；否则插入)
    async fn save_friendship(&self, friendship: Friends) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        // 插入或更新好友关系
        sqlx::query!(
            "INSERT INTO friends
            (fid, uid, to_uid, create_time, is_blacklist, to_is_blacklist, remark, to_remark, group_by, to_group_by)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON DUPLICATE KEY UPDATE
            create_time = VALUES(create_time),
            is_blacklist = VALUES(is_blacklist),
            to_is_blacklist = VALUES(to_is_blacklist),
            remark = VALUES(remark),
            to_remark = VALUES(to_remark),
            group_by = VALUES(group_by),
            to_group_by = VALUES(to_group_by)",
            friendship.fid,
            friendship.uid,
            friendship.to_uid,
            friendship.create_time,
            friendship.is_blacklist,
            friendship.to_is_blacklist,
            friendship.remark,
            friendship.to_remark,
            friendship.group_by,
            friendship.to_group_by,
        ).execute(&mut *tx).await?;

        // 提交事务
        tx.commit().await?;

        Ok(())
    }

    // 根据fid查找好友关系
    async fn find_friendship_by_fid(&self, fid: &str) -> AppResult<Option<Friends>> {
        let friendship = sqlx::query_as!(
            Friends,
            "SELECT
                fid, uid, to_uid, create_time, is_blacklist, to_is_blacklist,
                remark, to_remark, group_by, to_group_by
            FROM friends WHERE fid = ?",
            fid
        ).fetch_optional(self).await?;

        Ok(friendship)
    }

    // 根据两个uid查找好友关系
    async fn find_friendship_by_users(&self, uid1: &str, uid2: &str) -> AppResult<Option<Friends>> {
        // 确保较小的uid在前，较大的uid在后（保证数据一致性）
        let (smaller_uid, larger_uid) = if uid1 < uid2 {
            (uid1, uid2)
        } else {
            (uid2, uid1)
        };

        let friendship = sqlx::query_as!(
            Friends,
            "SELECT
                fid, uid, to_uid, create_time, is_blacklist, to_is_blacklist,
                remark, to_remark, group_by, to_group_by
            FROM friends
            WHERE uid = ? AND to_uid = ?",
            smaller_uid,
            larger_uid
        ).fetch_optional(self).await?;

        Ok(friendship)
    }

    // 查找一个用户的好友关系
    async fn find_friendship_by_uid(&self, uid: &str) -> AppResult<Vec<Friends>> {
        let friendships = sqlx::query_as!(
            Friends,
            "SELECT
                fid, uid, to_uid, create_time, is_blacklist, to_is_blacklist,
                remark, to_remark, group_by, to_group_by
            FROM friends
            WHERE uid = ? OR to_uid = ?",
            uid,
            uid
        ).fetch_all(self).await?;

        Ok(friendships)
    }

    // 删除好友关系
    async fn delete_friendship(&self, fid: &str) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "DELETE FROM friends WHERE fid = ?",
            fid
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    //-------------------------黑名单管理----------------------------
    // 保存记录到黑名单(可以加入黑名单也可以移出黑名单)
    async fn save_blacklist(&self, fid: &str, uid: &str, is_blacklist: bool) -> AppResult<()> {
        // 首先根据fid查找好友关系
        let friendship = self.find_friendship_by_fid(fid).await?;

        match friendship {
            Some(friend) => {
                // 判断uid是哪个字段
                if uid == friend.uid {
                    // uid与uid字段相同，更新is_blacklist
                    let blacklist_value = if is_blacklist { 1 } else { 0 };

                    // 事务
                    let mut tx = self.begin().await?;

                    sqlx::query!(
                        "UPDATE friends
                        SET is_blacklist = ?
                        WHERE fid = ?",
                        blacklist_value,
                        fid
                    ).execute(&mut *tx).await?;

                    tx.commit().await?;
                } else if uid == friend.to_uid {
                    // uid与to_uid字段相同，更新to_is_blacklist
                    let blacklist_value = if is_blacklist { 1 } else { 0 };

                    // 事务
                    let mut tx = self.begin().await?;

                    sqlx::query!(
                        "UPDATE friends
                        SET to_is_blacklist = ?
                        WHERE fid = ?",
                        blacklist_value,
                        fid
                    ).execute(&mut *tx).await?;

                    tx.commit().await?;
                } else {
                    // uid不匹配，返回错误
                    return Err(crate::models::errors::AppError::NotFound(
                        format!("User {} is not part of friendship {}", uid, fid)
                    ));
                }

                Ok(())
            }
            None => {
                Err(crate::models::errors::AppError::NotFound(
                    format!("Friendship {} not found", fid)
                ))
            }
        }
    }

    // 查找用户黑名单
    async fn find_blacklisted_friends(&self, uid: &str) -> AppResult<Vec<Friends>> {
        let blacklisted_friends = sqlx::query_as!(
            Friends,
            "SELECT
                fid, uid, to_uid, create_time, is_blacklist, to_is_blacklist,
                remark, to_remark, group_by, to_group_by
            FROM friends
            WHERE (uid = ? AND is_blacklist = 1) OR (to_uid = ? AND to_is_blacklist = 1)",
            uid,
            uid
        ).fetch_all(self).await?;

        Ok(blacklisted_friends)
    }

    //-------------------------好友申请管理----------------------------
    // 保存好友申请
    async fn save_friend_request(&self, request: FriendRequest) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "INSERT INTO friend_request
            (req_id, sender_uid, receiver_uid, status, apply_text, create_time, handle_time)
            VALUES (?, ?, ?, ?, ?, ?, ?)",
            request.req_id,
            request.sender_uid,
            request.receiver_uid,
            request.status,
            request.apply_text,
            request.create_time,
            request.handle_time
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }

    // 根据req_id查找好友申请
    async fn find_friend_request_by_id(&self, req_id: &str) -> AppResult<Option<FriendRequest>> {
        let request = sqlx::query_as!(
            FriendRequest,
            "SELECT
                req_id,
                sender_uid,
                receiver_uid,
                status  as `status: ReqStatus`,
                apply_text,
                create_time,
                handle_time
            FROM friend_request WHERE req_id = ?",
            req_id
        ).fetch_optional(self).await?;

        Ok(request)
    }

    // 根据接收者查找好友申请
    async fn find_friend_request_by_receiver(&self, receiver_uid: &str) -> AppResult<Vec<FriendRequest>> {
        let requests = sqlx::query_as!(
            FriendRequest,
            "SELECT
                req_id,
                sender_uid,
                receiver_uid,
                status  as `status: ReqStatus`,
                apply_text,
                create_time,
                handle_time
            FROM friend_request
            WHERE receiver_uid = ?
            ORDER BY create_time DESC",
            receiver_uid
        ).fetch_all(self).await?;

        Ok(requests)
    }

    // 根据发送者查找好友申请
    async fn find_friend_request_by_sender(&self, sender_uid: &str) -> AppResult<Vec<FriendRequest>> {
        let requests = sqlx::query_as!(
            FriendRequest,
            "SELECT
                req_id,
                sender_uid,
                receiver_uid,
                status  as `status: ReqStatus`,
                apply_text,
                create_time,
                handle_time
            FROM friend_request
            WHERE sender_uid = ?
            ORDER BY create_time DESC",
            sender_uid
        ).fetch_all(self).await?;

        Ok(requests)
    }

    // 更新好友申请状态
    async fn update_request_status(&self, req_id: &str, status: &str, handle_time: DateTime<Utc>) -> AppResult<()> {
        // 事务
        let mut tx = self.begin().await?;

        sqlx::query!(
            "UPDATE friend_request
            SET status = ?, handle_time = ?
            WHERE req_id = ?",
            status,
            handle_time,
            req_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}