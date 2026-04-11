//! Task Scheduler - Cron-based scheduled tasks for RustyLox
//!
//! Provides:
//! - Cron expression parsing and next-run calculation
//! - Built-in task types (backup, log rotation, health check)
//! - Custom script execution
//! - Task execution history
//! - Persistent configuration (scheduled_tasks.json)

pub mod config;
pub mod cron;
pub mod executor;
pub mod scheduler;

pub use config::{ScheduledTask, ScheduledTasksConfig, TaskType};
pub use cron::{describe_cron, Cron, CronBuilder, CronExpr, Weekday};
pub use executor::{ExecutionStatus, TaskExecution, TaskExecutor};
pub use scheduler::TaskScheduler;
