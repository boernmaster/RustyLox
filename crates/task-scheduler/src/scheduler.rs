//! Task scheduler - runs tasks based on cron schedules

use crate::config::{ScheduledTask, ScheduledTasksConfig, ScheduledTasksConfigManager};
use crate::executor::{TaskExecution, TaskExecutor};
use rustylox_core::Result;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Maximum number of execution history records to keep in memory
const MAX_HISTORY: usize = 100;

/// The task scheduler
pub struct TaskScheduler {
    config_manager: ScheduledTasksConfigManager,
    executor: TaskExecutor,
    /// Recent execution history (in-memory, last MAX_HISTORY entries)
    history: Arc<RwLock<VecDeque<TaskExecution>>>,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(lbhomedir: impl Into<PathBuf>) -> Self {
        let lbhomedir = lbhomedir.into();
        Self {
            config_manager: ScheduledTasksConfigManager::new(&lbhomedir),
            executor: TaskExecutor::new(&lbhomedir),
            history: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    /// Load the current task configuration
    pub async fn load_config(&self) -> Result<ScheduledTasksConfig> {
        self.config_manager.load().await
    }

    /// Save task configuration
    pub async fn save_config(&self, config: &ScheduledTasksConfig) -> Result<()> {
        self.config_manager.save(config).await
    }

    /// Run all tasks that are due now (for manual trigger / testing)
    pub async fn run_due_tasks(&self) -> Vec<TaskExecution> {
        let config = match self.load_config().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to load task config: {}", e);
                return Vec::new();
            }
        };

        let mut results = Vec::new();
        for task in config.tasks.iter().filter(|t| t.enabled) {
            if self.is_due(task) {
                let execution = self.run_task(task).await;
                results.push(execution);
            }
        }

        results
    }

    /// Manually trigger a specific task by ID
    pub async fn run_task_by_id(&self, task_id: &str) -> Result<TaskExecution> {
        let config = self.load_config().await?;
        let task = config
            .tasks
            .iter()
            .find(|t| t.id == task_id)
            .ok_or_else(|| rustylox_core::Error::config(format!("Task '{}' not found", task_id)))?
            .clone();

        Ok(self.run_task(&task).await)
    }

    /// Execute a specific task and record the result
    pub async fn run_task(&self, task: &ScheduledTask) -> TaskExecution {
        let execution = self.executor.execute(task).await;

        // Record in history
        {
            let mut history = self.history.write().await;
            if history.len() >= MAX_HISTORY {
                history.pop_front();
            }
            history.push_back(execution.clone());
        }

        execution
    }

    /// Get execution history
    pub async fn get_history(&self) -> Vec<TaskExecution> {
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }

    /// Get recent execution history (last N entries)
    pub async fn get_recent_history(&self, n: usize) -> Vec<TaskExecution> {
        let history = self.history.read().await;
        history.iter().rev().take(n).cloned().collect()
    }

    /// Start the background scheduler loop
    /// This spawns a tokio task that checks for due tasks every minute.
    pub fn start_background_scheduler(self: Arc<Self>) {
        info!("Starting background task scheduler");
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                debug!("Scheduler tick: checking for due tasks");
                let _ = self.run_due_tasks().await;
            }
        });
    }

    /// Check if a task is due to run now (within the current minute)
    fn is_due(&self, task: &ScheduledTask) -> bool {
        use cron::Schedule;
        use std::str::FromStr;

        let schedule = match Schedule::from_str(&task.schedule) {
            Ok(s) => s,
            Err(_) => return false,
        };

        // Check if the task should have run in the last 60 seconds
        let now = chrono::Utc::now();
        let one_minute_ago = now - chrono::Duration::seconds(60);

        schedule
            .after(&one_minute_ago)
            .next()
            .map(|next| next <= now)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_load_default_config() {
        let temp = TempDir::new().unwrap();
        let scheduler = TaskScheduler::new(temp.path());
        let config = scheduler.load_config().await.unwrap();
        // Should have default tasks
        assert!(!config.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_history_empty_initially() {
        let temp = TempDir::new().unwrap();
        let scheduler = TaskScheduler::new(temp.path());
        let history = scheduler.get_history().await;
        assert!(history.is_empty());
    }
}
