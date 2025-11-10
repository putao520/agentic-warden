//! 任务追踪器模块
//!
//! 用于追踪并发启动的 AI CLI 任务进程

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};

/// 任务信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskInfo {
    /// 任务ID
    pub task_id: String,
    /// AI CLI 类型
    pub ai_type: String,
    /// Provider
    pub provider: Option<String>,
    /// 任务内容
    pub task: String,
    /// 进程ID
    pub pid: u32,
    /// 启动时间
    pub started_at: DateTime<Utc>,
    /// 任务状态
    pub status: TaskStatus,
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 失败
    Failed(String),
}

/// 任务批次
#[derive(Debug, Clone)]
pub struct TaskBatch {
    /// 批次ID
    pub batch_id: String,
    /// 任务列表
    pub tasks: Vec<TaskInfo>,
    /// 创建时间
    pub created_at: DateTime<Utc>,
}

/// 全局任务追踪器
#[derive(Clone)]
pub struct TaskTracker {
    /// 当前任务批次
    current_batch: Arc<Mutex<Option<TaskBatch>>>,
    /// 历史批次（最多保留10个）
    history: Arc<Mutex<Vec<TaskBatch>>>,
}

impl TaskTracker {
    /// 创建新的任务追踪器
    pub fn new() -> Self {
        Self {
            current_batch: Arc::new(Mutex::new(None)),
            history: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 开始新的任务批次
    pub fn start_batch(&self, tasks: Vec<TaskInfo>) -> String {
        let batch_id = uuid::Uuid::new_v4().to_string();
        let batch = TaskBatch {
            batch_id: batch_id.clone(),
            tasks,
            created_at: Utc::now(),
        };

        let mut current = self.current_batch.lock().unwrap();

        // 如果有旧批次，移到历史记录
        if let Some(old_batch) = current.take() {
            let mut history = self.history.lock().unwrap();
            history.push(old_batch);

            // 只保留最近10个批次
            if history.len() > 10 {
                history.remove(0);
            }
        }

        *current = Some(batch);
        batch_id
    }

    /// 获取当前批次
    pub fn get_current_batch(&self) -> Option<TaskBatch> {
        self.current_batch.lock().unwrap().clone()
    }

    /// 获取当前批次的所有进程ID
    pub fn get_current_pids(&self) -> Vec<u32> {
        self.current_batch
            .lock()
            .unwrap()
            .as_ref()
            .map(|batch| batch.tasks.iter().map(|t| t.pid).collect())
            .unwrap_or_default()
    }

    /// 更新任务状态
    pub fn update_task_status(&self, pid: u32, status: TaskStatus) {
        if let Some(batch) = self.current_batch.lock().unwrap().as_mut() {
            if let Some(task) = batch.tasks.iter_mut().find(|t| t.pid == pid) {
                task.status = status;
            }
        }
    }

    /// 检查所有任务是否完成
    pub fn all_tasks_completed(&self) -> bool {
        self.current_batch
            .lock()
            .unwrap()
            .as_ref()
            .map(|batch| {
                batch.tasks.iter().all(|t| {
                    matches!(t.status, TaskStatus::Completed | TaskStatus::Failed(_))
                })
            })
            .unwrap_or(true)
    }

    /// 清除当前批次
    pub fn clear_current_batch(&self) {
        let mut current = self.current_batch.lock().unwrap();
        if let Some(batch) = current.take() {
            let mut history = self.history.lock().unwrap();
            history.push(batch);

            if history.len() > 10 {
                history.remove(0);
            }
        }
    }

    /// 获取运行中的任务数量
    pub fn get_running_count(&self) -> usize {
        self.current_batch
            .lock()
            .unwrap()
            .as_ref()
            .map(|batch| {
                batch.tasks.iter().filter(|t| t.status == TaskStatus::Running).count()
            })
            .unwrap_or(0)
    }

    /// 获取任务统计信息
    pub fn get_stats(&self) -> TaskStats {
        let batch = self.current_batch.lock().unwrap();

        if let Some(batch) = batch.as_ref() {
            let total = batch.tasks.len();
            let running = batch.tasks.iter().filter(|t| t.status == TaskStatus::Running).count();
            let completed = batch.tasks.iter().filter(|t| t.status == TaskStatus::Completed).count();
            let failed = batch.tasks.iter().filter(|t| matches!(t.status, TaskStatus::Failed(_))).count();

            TaskStats {
                total,
                running,
                completed,
                failed,
            }
        } else {
            TaskStats::default()
        }
    }
}

impl Default for TaskTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// 任务统计信息
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TaskStats {
    pub total: usize,
    pub running: usize,
    pub completed: usize,
    pub failed: usize,
}
