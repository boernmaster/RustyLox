//! Scheduled tasks UI handler

use crate::templates::{TaskEditTemplate, TaskForm, TasksTemplate};
use askama::Template;
use axum::{
    extract::{Path, State},
    response::Html,
    Form,
};
use serde::Deserialize;
use task_scheduler::{ScheduledTask, TaskScheduler};
use web_api::AppState;

/// Scheduled tasks page
pub async fn index(State(state): State<AppState>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let template = TasksTemplate {
        version: state.version.clone(),
        lang,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

/// Show edit form for a scheduled task
pub async fn edit_form(State(state): State<AppState>, Path(id): Path<String>) -> Html<String> {
    let lang = state.config.read().await.base.lang.clone();
    let scheduler = TaskScheduler::new(&state.lbhomedir, &state.version);

    let task_form = match scheduler.load_config().await {
        Ok(config) => config.tasks.into_iter().find(|t| t.id == id).map(|t| {
            let task_type = serde_json::to_value(&t.task_type)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_default();
            TaskForm {
                id: t.id,
                name: t.name,
                schedule: t.schedule,
                task_type,
                script_path: t.script_path.unwrap_or_default(),
                enabled: t.enabled,
            }
        }),
        Err(_) => None,
    };

    let template = TaskEditTemplate {
        task: task_form,
        version: state.version.clone(),
        lang,
    };

    Html(
        template
            .render()
            .unwrap_or_else(|_| "Error rendering template".to_string()),
    )
}

#[derive(Debug, Deserialize)]
pub struct TaskEditFormData {
    pub name: String,
    pub schedule: String,
    pub script_path: Option<String>,
    /// Checkbox: submitted as "on" when checked, absent when unchecked
    pub enabled: Option<String>,
}

/// Submit edited scheduled task
pub async fn edit_submit(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Form(form): Form<TaskEditFormData>,
) -> Html<String> {
    let scheduler = TaskScheduler::new(&state.lbhomedir, &state.version);

    let mut config = match scheduler.load_config().await {
        Ok(c) => c,
        Err(e) => {
            return Html(format!(
                "<div class='alert alert-danger'>Failed to load config: {}. \
                 <a href='/tasks'>Back to tasks</a></div>",
                e
            ))
        }
    };

    if !ScheduledTask::is_valid_schedule(&form.schedule) {
        return Html(format!(
            "<div class='alert alert-danger'>Invalid cron expression: '{}'. \
             <a href='/tasks/{}/edit'>Go back</a></div>",
            form.schedule, id
        ));
    }

    let task = match config.tasks.iter_mut().find(|t| t.id == id) {
        Some(t) => t,
        None => {
            return Html(format!(
                "<div class='alert alert-danger'>Task '{}' not found. \
                 <a href='/tasks'>Back to tasks</a></div>",
                id
            ))
        }
    };

    task.name = form.name;
    task.schedule = form.schedule;
    task.enabled = form.enabled.map(|v| v == "on").unwrap_or(false);
    if let Some(ref path) = form.script_path {
        if !path.is_empty() {
            task.script_path = Some(path.clone());
        } else {
            task.script_path = None;
        }
    }

    match scheduler.save_config(&config).await {
        Ok(_) => Html(
            "<div class='alert alert-success'>Task updated successfully. \
             <a href='/tasks'>Back to tasks</a></div>"
                .to_string(),
        ),
        Err(e) => Html(format!(
            "<div class='alert alert-danger'>Error saving: {}. \
             <a href='/tasks'>Back to tasks</a></div>",
            e
        )),
    }
}
