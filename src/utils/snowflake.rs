use std::{thread, time::{Duration, SystemTime, UNIX_EPOCH}};
use std::sync::Mutex;
use crate::models::errors::{AppError, AppResult};

pub struct Snowflake {
    // 起始时间戳
    epoch: i64,
    // 机器ID
    machine_id: i64,
    // 需要同步原语的变量
    state: Mutex<SnowflakeState>
}

struct SnowflakeState {
    // 序列号
    sequence: i64,
    // 上次生成时间
    last_timestamp: i64
}

impl Snowflake {
    // 一些要用到的常量
    // 1位符号位+41位时间戳+10位机器id+12位序列号
    const MACHINE_ID_BITS: i64 = 10;
    const SEQUENCE_BITS: i64 = 12;
    // 最大值
    const MAX_MACHINE_ID: i64 = (1 << Self::MACHINE_ID_BITS) - 1;
    const MAX_SEQUENCE: i64 = (1 << Self::SEQUENCE_BITS) - 1;
    // 偏移量
    const MACHINE_ID_SHIFT: i64 = Self::SEQUENCE_BITS;
    const TIMESTAMP_SHIFT: i64 = Self::MACHINE_ID_BITS + Self::SEQUENCE_BITS;

    // 构造函数
    pub fn new(machine_id: i64, epoch: Option<i64>) -> AppResult<Self> {
        // 判断machine_id是否合法
        if machine_id < 0 || machine_id > Self::MAX_MACHINE_ID {
            return Err(AppError::SnowflakeFailure("机器ID有误".to_string()));
        }
        
        // 判断是否有初始时间戳，没有则默认值
        let epoch = epoch.unwrap_or(1_577_836_800_000);

        // 初始化序列号和上次生成时间
        Ok(Self {
            epoch,
            machine_id,
            state: Mutex::new(SnowflakeState {
                sequence: 0,
                last_timestamp: -1,
            })
        })
    }

    // 创建雪花ID函数
    pub fn next_id(&self) -> AppResult<i64> {
        // 获取状态的锁
        let mut state = self.state.lock()
            .map_err(|e| AppError::SnowflakeFailure(e.to_string()))?;
        // 先获取当前时间戳
        let mut timestamp = self.current_timestamp()?;
        // 获取上次生成时间戳
        let last_timestamp = state.last_timestamp;
        // 获取当前序列
        let mut sequence = state.sequence;
        // 对比上次生成时间戳，如果时钟回拨则处理
        if timestamp < last_timestamp {
            return Err(AppError::SnowflakeFailure("时钟回拨，拒绝生成ID".to_string()));
        }
        // 如果时钟没走，则序列加1，如果序列走了，则序列重置
        if timestamp == last_timestamp {
            // 检测序列是否到达上限，如果到达上限则等待下一毫秒
            sequence = (sequence + 1) & Self::MAX_SEQUENCE;
            if sequence == 0 {
                timestamp = self.wait_next_millis(timestamp)?;
            }
        }
        else {
            sequence = 0;
        }
        // 更新最后时间戳以及序列
        state.last_timestamp = timestamp;
        state.sequence = sequence;
        // 组合ID
        let id = ((timestamp - self.epoch) << Self::TIMESTAMP_SHIFT)
            | (self.machine_id << Self::MACHINE_ID_SHIFT)
            | sequence;
        
        Ok(id)
    }

    // 获取当前时间戳
    pub fn current_timestamp(&self) -> AppResult<i64> {
        let result = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_err(|e| AppError::SnowflakeFailure(e.to_string()))?
                .as_millis() as i64;
        Ok(result)
    }

    // 等待下一毫秒，用于sequence上限
    pub fn wait_next_millis(&self, last_timestamp: i64) -> AppResult<i64> {
        let mut timestamp = self.current_timestamp()?;
        let mut retries = 0;
        const MAX_RETRIES:u32 = 50;
        // 一直等到下一毫秒
        while timestamp <= last_timestamp {
            if retries >= MAX_RETRIES {
                return Err(AppError::SnowflakeFailure("等待时间戳递增超时，系统时间异常？".to_string()));
            }

            thread::sleep(Duration::from_millis(1));
            timestamp = self.current_timestamp()?;
            retries += 1;
        }
        Ok(timestamp)
    }
}