//! Scheduled task management API endpoints

use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use task_scheduler::{ScheduledTask, TaskScheduler, TaskType};
use tracing::{error, info};

/// Request to create a new task
#[derive(Debug, Deserialize)]
pub struct CreateTaskRequest {
    pub name: String,
    pub schedule: String,
    pub task_type: String,
    pub script_path: Option<String>,
    pub enabled: Option<bool>,
}

/// Request to update a task
#[derive(Debug, Deserialize)]
pub struct UpdateTaskRequest {
    pub name: Option<String>,
    pub schedule: Option<String>,
    pub enabled: Option<bool>,
    pub script_path: Option<String>,
}

/// List all scheduled tasks
///
/// GET /api/tasks
pub async fn list_tasks(State(state): State<AppState>) -> impl IntoResponse {
    let scheduler = TaskScheduler::new(&state.lbhomedir);
    match scheduler.load_config().await {
        Ok(config) => {
            // Enrich with next run time
            let tasks_with_next: Vec<serde_json::Value> = config
                .tasks
                .iter()
                .map(|t| {
                    let next_run = t.next_run().map(|dt| dt.to_rfc3339());
                    let mut val = serde_json::to_value(t).unwrap_or_default();
                    if let serde_json::Value::Object(ref mut map) = val {
                        map.insert(
                            "next_run".to_string(),
                            next_run
                                .map(serde_json::Value::String)
                                .unwrap_or(serde_json::Value::Null),
                        );
                    }
                    val
                })
                .collect();

            Json(serde_json::json!({
                "tasks": tasks_with_next,
                "count": tasks_with_next.len()
            }))
            .into_response()
        }
        Err(e) => {
            error!("Failed to list tasks: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response()
        }
    }
}

/// Get a single task by ID
///
/// GET /api/tasks/:id
pub async fn get_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    let scheduler = TaskScheduler::new(&state.lbhomedir);
    match scheduler.load_config().await {
        Ok(config) => match config.tasks.iter().find(|t| t.id == task_id) {
            Some(task) => Json(task).into_response(),
            None => (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Task '{}' not found", task_id) })),
            )
                .into_response(),
        },
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// Create a new scheduled task
///
/// POST /api/tasks
pub async fn create_task(
    State(state): State<AppState>,
    Json(req): Json<CreateTaskRequest>,
) -> impl IntoResponse {
    // Validate schedule
    if !ScheduledTask::is_valid_schedule(&req.schedule) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({ "error": format!("Invalid cron expression: '{}'", req.schedule) })),
        )
            .into_response();
    }

    // Parse task type
    let task_type = match req.task_type.as_str() {
        "backup" => TaskType::Backup,
        "log_rotation" => TaskType::LogRotation,
        "health_check" => TaskType::HealthCheck,
        "custom" => TaskType::Custom,
        other => {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Unknown task type: '{}'", other) })),
            )
                .into_response();
        }
    };

    let mut task = ScheduledTask::new(&req.name, &req.schedule, task_type);
    task.script_path = req.script_path;
    task.enabled = req.enabled.unwrap_or(true);

    let scheduler = TaskScheduler::new(&state.lbhomedir);
    let mut config = match scheduler.load_config().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response();
        }
    };

    let task_id = task.id.clone();
    config.tasks.push(task.clone());

    match scheduler.save_config(&config).await {
        Ok(_) => {
            info!("Created task: {}", task_id);
            (StatusCode::CREATED, Json(task)).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// Update a scheduled task
///
/// PUT /api/tasks/:id
pub async fn update_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Json(req): Json<UpdateTaskRequest>,
) -> impl IntoResponse {
    let scheduler = TaskScheduler::new(&state.lbhomedir);
    let mut config = match scheduler.load_config().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response();
        }
    };

    let task = match config.tasks.iter_mut().find(|t| t.id == task_id) {
        Some(t) => t,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(serde_json::json!({ "error": format!("Task '{}' not found", task_id) })),
            )
                .into_response();
        }
    };

    // Validate new schedule if provided
    if let Some(ref schedule) = req.schedule {
        if !ScheduledTask::is_valid_schedule(schedule) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": format!("Invalid cron expression: '{}'", schedule) })),
            )
                .into_response();
        }
        task.schedule = schedule.clone();
    }

    if let Some(name) = req.name {
        task.name = name;
    }
    if let Some(enabled) = req.enabled {
        task.enabled = enabled;
    }
    if let Some(script_path) = req.script_path {
        task.script_path = Some(script_path);
    }

    let updated_task = task.clone();

    match scheduler.save_config(&config).await {
        Ok(_) => Json(updated_task).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// Delete a scheduled task
///
/// DELETE /api/tasks/:id
pub async fn delete_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    let scheduler = TaskScheduler::new(&state.lbhomedir);
    let mut config = match scheduler.load_config().await {
        Ok(c) => c,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response();
        }
    };

    let original_len = config.tasks.len();
    config.tasks.retain(|t| t.id != task_id);

    if config.tasks.len() == original_len {
        return (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": format!("Task '{}' not found", task_id) })),
        )
            .into_response();
    }

    match scheduler.save_config(&config).await {
        Ok(_) => {
            info!("Deleted task: {}", task_id);
            Json(serde_json::json!({ "success": true })).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "error": format!("{}", e) })),
        )
            .into_response(),
    }
}

/// Manually trigger a task
///
/// POST /api/tasks/:id/run
pub async fn run_task(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
) -> impl IntoResponse {
    info!("Manually triggering task: {}", task_id);
    let scheduler = TaskScheduler::new(&state.lbhomedir);

    match scheduler.run_task_by_id(&task_id).await {
        Ok(execution) => Json(execution).into_response(),
        Err(e) => {
            error!("Failed to run task {}: {}", task_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "error": format!("{}", e) })),
            )
                .into_response()
        }
    }
}

/// Get task execution history
///
/// GET /api/tasks/history
pub async fn get_history(State(state): State<AppState>) -> impl IntoResponse {
    let scheduler = TaskScheduler::new(&state.lbhomedir);
    let history = scheduler.get_recent_history(50).await;
    Json(serde_json::json!({
        "history": history,
        "count": history.len()
    }))
}
