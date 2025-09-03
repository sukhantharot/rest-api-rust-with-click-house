use crate::database::DatabasePool;
use crate::models::task::*;
use crate::services::{register_builtin_handlers, TaskService};
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct TaskListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub status: Option<String>,
}

pub fn task_routes() -> Router<DatabasePool> {
    Router::new()
        .route("/tasks", get(get_tasks))
        .route("/tasks", post(create_task))
        .route("/tasks/{id}", get(get_task))
        .route("/tasks/{id}/cancel", post(cancel_task))
        .route("/tasks/maintenance/cleanup", post(schedule_cleanup))
        .route("/tasks/stats", get(get_task_stats))
}

async fn create_task(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<Json<TaskResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
    let task_service = TaskService::new();

    // Register built-in handlers if not already registered
    register_builtin_handlers(&task_service).await;

    match task_service
        .create_task(&pool, &domain, payload, None)
        .await
    {
        Ok(task) => Ok(Json(task)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn get_tasks(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<TaskListQuery>,
) -> Result<Json<Vec<TaskResponse>>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
    let task_service = TaskService::new();

    // For now, we'll just return pending tasks
    // In a full implementation, you'd filter by status
    match task_service
        .get_pending_tasks(&pool, &domain, query.limit)
        .await
    {
        Ok(tasks) => Ok(Json(tasks)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_task(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
    let task_service = TaskService::new();

    match task_service.get_task_by_id(&pool, &domain, task_id).await {
        Ok(task) => Ok(Json(task)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Task not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn cancel_task(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(task_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
    let task_service = TaskService::new();

    match task_service.cancel_task(&pool, &domain, task_id).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Task not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn schedule_cleanup(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<TaskResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
    let task_service = TaskService::new();

    // Register built-in handlers
    register_builtin_handlers(&task_service).await;

    // Schedule a cleanup task for old tracking data
    let cleanup_payload = CleanupTaskPayload {
        table_name: "auth_tracking".to_string(),
        older_than_days: 30,
        batch_size: Some(1000),
    };

    let cleanup_request = CreateTaskRequest {
        task_type: task_types::DATA_CLEANUP.to_string(),
        payload: serde_json::to_value(cleanup_payload)
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
        priority: Some(TaskPriority::Low),
        scheduled_at: Some(chrono::Utc::now()),
        max_attempts: Some(3),
    };

    match task_service
        .create_task(&pool, &domain, cleanup_request, None)
        .await
    {
        Ok(task) => Ok(Json(task)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_task_stats(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<TaskStatsResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;

    // This is a simplified stats implementation
    // In a real system, you'd query the database for actual statistics
    let stats = TaskStatsResponse {
        total_tasks: 0,
        pending_tasks: 0,
        running_tasks: 0,
        completed_tasks: 0,
        failed_tasks: 0,
        average_execution_time: 0.0,
    };

    Ok(Json(stats))
}

#[derive(serde::Serialize)]
pub struct TaskStatsResponse {
    pub total_tasks: u64,
    pub pending_tasks: u64,
    pub running_tasks: u64,
    pub completed_tasks: u64,
    pub failed_tasks: u64,
    pub average_execution_time: f64,
}

fn extract_domain_from_headers(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    let host = headers
        .get("host")
        .or_else(|| headers.get("x-tenant-domain"))
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing domain information".to_string(),
            )
        })?;

    let domain = host.split(':').next().unwrap_or(host);
    Ok(domain.to_string())
}
