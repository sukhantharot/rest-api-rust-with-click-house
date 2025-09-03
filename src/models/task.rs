use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub task_type: String,
    pub payload: String, // JSON payload
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub max_attempts: u32,
    pub attempts: u32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_by: Option<Uuid>,
    pub domain: String, // For multi-tenant support
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTaskRequest {
    pub task_type: String,
    pub payload: serde_json::Value,
    pub priority: Option<TaskPriority>,
    pub scheduled_at: Option<DateTime<Utc>>,
    pub max_attempts: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateTaskRequest {
    pub status: Option<TaskStatus>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    pub id: Uuid,
    pub task_type: String,
    pub payload: serde_json::Value,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub max_attempts: u32,
    pub attempts: u32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_by: Option<Uuid>,
    pub domain: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct TaskExecution {
    pub id: Uuid,
    pub task_id: Uuid,
    pub worker_id: String,
    pub status: TaskExecutionStatus,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub duration_ms: Option<u64>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskStatus {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "retry")]
    Retry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum TaskPriority {
    #[serde(rename = "low")]
    Low = 1,
    #[serde(rename = "normal")]
    Normal = 2,
    #[serde(rename = "high")]
    High = 3,
    #[serde(rename = "critical")]
    Critical = 4,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskExecutionStatus {
    #[serde(rename = "started")]
    Started,
    #[serde(rename = "completed")]
    Completed,
    #[serde(rename = "failed")]
    Failed,
}

// Built-in task types
pub mod task_types {
    pub const EMAIL_SEND: &str = "email.send";
    pub const DATA_CLEANUP: &str = "data.cleanup";
    pub const REPORT_GENERATE: &str = "report.generate";
    pub const CACHE_INVALIDATE: &str = "cache.invalidate";
    pub const BACKUP_CREATE: &str = "backup.create";
    pub const NOTIFICATION_SEND: &str = "notification.send";
}

// Task payload structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTaskPayload {
    pub to: String,
    pub subject: String,
    pub body: String,
    pub html: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupTaskPayload {
    pub table_name: String,
    pub older_than_days: u32,
    pub batch_size: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportTaskPayload {
    pub report_type: String,
    pub parameters: serde_json::Value,
    pub recipient_email: Option<String>,
}
