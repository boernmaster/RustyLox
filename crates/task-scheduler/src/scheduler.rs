//! Task scheduler - runs tasks based on cron schedules

use crate::config::{ScheduledTask, ScheduledTasksConfig, ScheduledTasksConfigManager};
use crate::executor::{TaskExecution, TaskExecutor};
use rustylox_core::Result;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Maximum number of execution history records to keep
const MAX_HISTORY: usize = 100;

/// The task scheduler
pub struct TaskScheduler {
    config_manager: ScheduledTasksConfigManager,
    executor: TaskExecutor,
    /// Recent execution history (in-memory, last MAX_HISTORY entries)
    history: Arc<RwLock<VecDeque<TaskExecution>>>,
    /// Path for persisting history to disk
    history_path: PathBuf,
}

impl TaskScheduler {
    /// Create a new task scheduler
    pub fn new(lbhomedir: impl Into<PathBuf>, version: impl Into<String>) -> Self {
        let lbhomedir = lbhomedir.into();
        let history_path = lbhomedir.join("data/system/task_history.json");
        Self {
            config_manager: ScheduledTasksConfigManager::new(&lbhomedir),
            executor: TaskExecutor::new(&lbhomedir, version),
            history: Arc::new(RwLock::new(VecDeque::new())),
            history_path,
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

    /// Load execution history from disk into memory (only if memory is empty)
    async fn load_history_from_disk(&self) {
        {
            let history = self.history.read().await;
            if !history.is_empty() {
                return;
            }
        }
        if !self.history_path.exists() {
            return;
        }
        match tokio::fs::read_to_string(&self.history_path).await {
            Ok(content) => {
                if let Ok(entries) = serde_json::from_str::<Vec<TaskExecution>>(&content) {
                    let mut history = self.history.write().await;
                    if history.is_empty() {
                        *history = entries.into_iter().collect();
                    }
                }
            }
            Err(e) => {
                warn!("Failed to load task history from disk: {}", e);
            }
        }
    }

    /// Save execution history to disk
    async fn save_history_to_disk(&self) {
        let history = self.history.read().await;
        let entries: Vec<&TaskExecution> = history.iter().collect();
        if let Ok(content) = serde_json::to_string_pretty(&entries) {
            if let Some(parent) = self.history_path.parent() {
                let _ = tokio::fs::create_dir_all(parent).await;
            }
            if let Err(e) = tokio::fs::write(&self.history_path, content).await {
                warn!("Failed to save task history to disk: {}", e);
            }
        }
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
        self.load_history_from_disk().await;

        let execution = self.executor.execute(task).await;

        // Record in history
        {
            let mut history = self.history.write().await;
            if history.len() >= MAX_HISTORY {
                history.pop_front();
            }
            history.push_back(execution.clone());
        }

        self.save_history_to_disk().await;
        execution
    }

    /// Get execution history
    pub async fn get_history(&self) -> Vec<TaskExecution> {
        self.load_history_from_disk().await;
        let history = self.history.read().await;
        history.iter().cloned().collect()
    }

    /// Get recent execution history (last N entries)
    pub async fn get_recent_history(&self, n: usize) -> Vec<TaskExecution> {
        self.load_history_from_disk().await;
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
        let scheduler = TaskScheduler::new(temp.path(), "test");
        let config = scheduler.load_config().await.unwrap();
        assert!(!config.tasks.is_empty());
    }

    #[tokio::test]
    async fn test_history_empty_initially() {
        let temp = TempDir::new().unwrap();
        let scheduler = TaskScheduler::new(temp.path(), "test");
        let history = scheduler.get_history().await;
        assert!(history.is_empty());
    }
}
