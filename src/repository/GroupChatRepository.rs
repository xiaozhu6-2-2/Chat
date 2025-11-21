use std::mem;

use chrono::NaiveDateTime;
use sqlx::{MySqlPool, query_as};
use async_trait::async_trait;

use crate::models::errors::AppResult;
use crate::models::repository::GroupChatRepository;
use crate::models::entities::{GroupChat, GroupJoinRequest, GroupMember, MuteRecord};

#[async_trait]
impl GroupChatRepository for MySqlPool {
    //-------------------------群聊基础管理----------------------------
    // 保存群聊
    async fn save_group(&self, group: GroupChat) -> AppResult<()>{

        //事务
        let mut tx=self.begin().await?;

        // //先检查是否存在群聊
        // let exists=sqlx::query_as!(
        //     GroupChat,
        //     "SELECT * FROM group_chat WHERE gid = ?",
        //     group.gid
        // ).fetch_optional(&mut *tx).await?;

        // //若存在群聊则更新
        // match exists {
        //     Some(_) =>{
        //         sqlx::query!(
        //         "UPDATE group_chat 
        //             SET group_name = ?,
        //             manager_uid = ?,
        //             group_avatar = ?,
        //             group_intro = ?,
        //             create_time = ? 
        //         WHERE gid = ?",
        //         group.group_name,
        //         group.manager_uid,
        //         group.group_avatar,
        //         group.group_intro,
        //         group.create_time,
        //         group.gid
        //         ).execute(&mut *tx).await?;
        //     }
        //     //若不存在群聊则插入
        //     None =>{
        //         sqlx::query!(
        //         "INSERT INTO group_chat (gid, group_name, manager_uid, group_avatar, group_intro, create_time) VALUES (?,?,?,?,?,?)",
        //         group.gid,
        //         group.group_name,
        //         group.manager_uid,
        //         group.group_avatar,
        //         group.group_intro,
        //         group.create_time,
        //         ).execute(&mut *tx).await?;
        //     }
        // }
        
        //插入或更新
        sqlx::query!(
            "INSERT INTO group_chat 
            (gid, group_name, manager_uid, group_avatar, group_intro, create_time) 
            VALUES (?,?,?,?,?,?)
            ON DUPLICATE KEY UPDATE
            group_name = VALUES(group_name),
            manager_uid = VALUES(manager_uid),
            group_avatar = VALUES(group_avatar),
            group_intro = VALUES(group_intro),
            create_time = VALUES(create_time)
            ",
                group.gid,
                group.group_name,
                group.manager_uid,
                group.group_avatar,
                group.group_intro,
                group.create_time,
        ).execute(&mut *tx).await?;

        //提交事务
        tx.commit().await?;

        Ok(())
    }

    // 按gid查找群聊
    async fn find_group_by_gid(&self, gid: &str) -> AppResult<Option<GroupChat>>{

        let find_group=sqlx::query_as!(
            GroupChat,
            "SELECT * FROM group_chat WHERE gid = ?",
            gid
        ).fetch_optional(self).await?;

        Ok(find_group)
    }

    // 按照群主查找群聊
    async fn find_group_by_owner(&self, owner_uid: &str) -> AppResult<Vec<GroupChat>>{
        
        let find_group=sqlx::query_as!(
            GroupChat,
            "SELECT * FROM group_chat WHERE manager_uid = ?",
            owner_uid
        ).fetch_all(self).await?;
        
        Ok(find_group)
    }

    // 按照群名查找群聊
    async fn find_group_by_name(&self, name: &str) -> AppResult<Vec<GroupChat>>{
        
        let find_group=sqlx::query_as!(
            GroupChat,
            "SELECT gid,group_name,manager_uid,group_avatar,group_intro,create_time FROM group_chat WHERE group_name = ?",
            name
        ).fetch_all(self).await?;
        
        Ok(find_group)
    }

    // 删除群聊
    async fn delete_group(&self, gid: &str) -> AppResult<()>{

        //事务
        let mut tx=self.begin().await?;

        sqlx::query!(
            "DELETE FROM group_chat WHERE gid = ?",
            gid
        ).execute(&mut *tx).await?;

        tx.commit().await?;
        
        Ok(())
    }
    
//-------------------------群聊成员管理----------------------------
    // 加入群聊
    async fn save_member(&self, member: GroupMember) -> AppResult<()>{

        //事务
        let mut tx=self.begin().await?;

        //检查成员是否存在
        let exists=sqlx::query_as!(
            GroupMember,
            "SELECT * FROM group_member WHERE uid = ? AND gid = ?",
            member.uid,
            member.gid
        ).fetch_optional(&mut *tx).await?;


        //若存在成员记录则更新
        match exists {
            Some(_) => {
                sqlx::query!(
                "UPDATE group_member 
                    SET role = ?,
                    nickname = ?,
                    level = ?,
                    join_time = ?,
                    do_not_disturb = ?,
                    tag = ?,
                    remark = ?,
                    is_pinned = ?
                WHERE uid = ? AND gid = ?",
                member.role,
                member.nickname,
                member.level,
                member.join_time,
                member.do_not_disturb,
                member.tag,
                member.remark,
                member.is_pinned,
                member.uid,
                member.gid
                ).execute(&mut *tx).await?;
            }
        //不存在则插入记录
            None => {
                sqlx::query!(
                "INSERT INTO group_member (uid, gid, role, nickname, level, join_time, do_not_disturb, tag, remark, is_pinned) VALUES (?,?,?,?,?,?,?,?,?,?)",
                member.uid,
                member.gid,
                member.role,
                member.nickname,
                member.level,
                member.join_time,
                member.do_not_disturb,
                member.tag,
                member.remark,
                member.is_pinned,
                ).execute(&mut *tx).await?;
            }
        }

        tx.commit().await?;

        Ok(())
    }
    // 查找群成员
    async fn find_member(&self, gid: &str, uid: &str) -> AppResult<Option<GroupMember>>{

        let find_member=sqlx::query_as!(
            GroupMember,
            "SELECT * FROM group_member WHERE uid = ? AND gid = ?",
            uid,
            gid
        ).fetch_optional(self).await?;

        Ok(find_member)
    }
    // 查找群聊的群成员列表
    async fn find_members_by_group(&self, gid: &str) -> AppResult<Vec<GroupMember>>{

        let find_members=sqlx::query_as!(
            GroupMember,
            "SELECT * FROM group_member WHERE gid = ?",
            gid
        ).fetch_all(self).await?;

        Ok(find_members)
    }
    // 查找用户的群聊列表
    async fn find_groups_by_user(&self, uid: &str) -> AppResult<Vec<GroupMember>>{

        let find_group=sqlx::query_as!(
            GroupMember,
            "SELECT * FROM group_member WHERE uid = ?",
            uid
        ).fetch_all(self).await?;

        Ok(find_group)
    }
    // 更新用户权限
    async fn update_member_role(&self, role: &str, gid: &str, uid: &str) -> AppResult<()>{

        //事务
        let mut tx=self.begin().await?;

        sqlx::query!(
            "UPDATE group_member
                SET role = ?
            WHERE gid = ? AND uid = ?",
            role,
            gid,
            uid
        ).execute(&mut *tx).await?;

        tx.commit().await?;
        
        Ok(())
    }
    // 退出群聊
    async fn remove_member(&self, gid: &str, uid: &str) -> AppResult<()>{

        //事务
        let mut tx=self.begin().await?;

        sqlx::query!(
            "DELETE FROM group_member WHERE uid = ? AND gid = ?",
            uid,
            gid
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
//-------------------------禁言管理-------------------------------
    // 禁言用户
    async fn add_mute_record(&self, mute: MuteRecord) -> AppResult<()>{
        
        //事务
        let mut tx=self.begin().await?;

        sqlx::query!(
            "INSERT INTO mute_record 
            (ban_id, gid, uid, mute_duration, start_time) 
            VALUES (?,?,?,?,?)
            ON DUPLICATE KEY UPDATE
            mute_duration = VALUES(mute_duration),
            start_time = VALUES(start_time)
            ",
            mute.ban_id,
            mute.gid,
            mute.uid,
            mute.mute_duration,
            mute.start_time
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    // 查找群聊禁言名单
    async fn find_mute_records_by_group(&self, gid: &str) -> AppResult<Vec<MuteRecord>>{

        let find_mute=sqlx::query_as!(
            MuteRecord,
            "SELECT * FROM mute_record WHERE gid = ?",
            gid
        ).fetch_all(self).await?;

        Ok(find_mute)

    }
    // 查找用户被禁言记录
    async fn find_mute_records_by_user(&self, gid: &str, uid: &str) -> AppResult<Option<MuteRecord>>{

        let find_mute_user=sqlx::query_as!(
            MuteRecord,
            "SELECT * FROM mute_record WHERE gid = ? AND uid = ?",
            gid,
            uid
        ).fetch_optional(self).await?;

        Ok(find_mute_user)

    }
    // 解除禁言
    async fn remove_mute(&self, ban_id: &str) -> AppResult<()>{
        
        let mut tx=self.begin().await?;

        sqlx::query!(
            "DELETE FROM mute_record WHERE ban_id = ?",
            ban_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    // 用于清理过期禁言
    async fn find_expired_mute_records(&self) -> AppResult<Vec<MuteRecord>>{

        let mut tx=self.begin().await?;

        let expired_records = sqlx::query_as!(
            MuteRecord,
            "SELECT ban_id, gid, uid, mute_duration, start_time 
            FROM mute_record 
            WHERE start_time IS NOT NULL 
                AND DATE_ADD(start_time, INTERVAL mute_duration SECOND) < NOW()"
        ).fetch_all(&mut * tx).await?;

        sqlx::query!(
            "DELETE FROM mute_record WHERE start_time IS NOT NULL AND DATE_ADD(start_time, INTERVAL mute_duration SECOND) < NOW()"
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(expired_records)
    }
//-------------------------群聊申请管理-------------------------------
    // 保存群聊申请
    async fn save_group_request(&self, request: GroupJoinRequest) -> AppResult<()>{

        let mut tx=self.begin().await?;

        sqlx::query!(
            "INSERT INTO group_join_request(req_id, gid, applicant_uid, approver_uid, status, apply_text, create_time, handle_time) 
            VALUES (?,?,?,?,?,?,?,?)",
            request.req_id,
            request.gid,
            request.applicant_uid,
            request.approver_uid,
            request.status,
            request.apply_text,
            request.create_time,
            request.handle_time
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
    // 根据req_id查找群聊申请
    async fn find_group_request_by_id(&self, req_id: &str) -> AppResult<Option<GroupJoinRequest>>{

        let find_request=sqlx::query_as!(
            GroupJoinRequest,
            "SELECT * FROM group_join_request WHERE req_id = ?",
            req_id
        ).fetch_optional(self).await?;

        Ok(find_request)
    }
    // 查找群聊未处理申请
    async fn find_pending_requests_by_group(&self, gid: &str) -> AppResult<Vec<GroupJoinRequest>>{

        let find_request=sqlx::query_as!(
            GroupJoinRequest,
            "SELECT * FROM group_join_request WHERE gid = ? AND status = 'pending'",
            gid
        ).fetch_all(self).await?;

        Ok(find_request)
    }
    // 更新群聊申请状态
    async fn update_request_status(&self, req_id: &str, status: &str, approver_uid: &str, handle_time: NaiveDateTime) -> AppResult<()>{

        let mut tx=self.begin().await?;

        sqlx::query!(
            "UPDATE group_join_request
                SET status = ?,
                approver_uid = ?,
                handle_time = ?
            WHERE req_id = ?",
            status,
            approver_uid,
            handle_time,
            req_id
        ).execute(&mut *tx).await?;

        tx.commit().await?;

        Ok(())
    }
}