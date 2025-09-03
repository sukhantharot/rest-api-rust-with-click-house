use crate::database::DatabasePool;
use crate::models::task::*;
use chrono::{DateTime, Duration, Utc};
use clickhouse::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type TaskRegistry = Arc<RwLock<HashMap<String, Box<dyn TaskHandler + Send + Sync>>>>;

pub struct TaskService {
    task_registry: TaskRegistry,
}

impl TaskService {
    pub fn new() -> Self {
        Self {
            task_registry: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn get_registry(&self) -> TaskRegistry {
        Arc::clone(&self.task_registry)
    }

    pub async fn register_handler<H>(&self, task_type: &str, handler: H)
    where
        H: TaskHandler + Send + Sync + 'static,
    {
        let mut registry = self.task_registry.write().await;
        registry.insert(task_type.to_string(), Box::new(handler));
    }

    pub async fn create_task(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateTaskRequest,
        created_by: Option<Uuid>,
    ) -> Result<TaskResponse, anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let task_id = Uuid::new_v4();
        let now = Utc::now();
        let scheduled_at = request.scheduled_at.unwrap_or(now);
        let priority = request.priority.unwrap_or(TaskPriority::Normal);
        let max_attempts = request.max_attempts.unwrap_or(3);

        let payload_json = serde_json::to_string(&request.payload)?;

        // Insert task
        client
            .query(
                r#"
                INSERT INTO tasks (
                    id, task_type, payload, status, priority, max_attempts,
                    attempts, scheduled_at, created_by, domain, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(task_id)
            .bind(&request.task_type)
            .bind(&payload_json)
            .bind("pending")
            .bind(priority as u8)
            .bind(max_attempts)
            .bind(0u32)
            .bind(scheduled_at)
            .bind(created_by.map(|id| id.to_string()))
            .bind(domain)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        // Get the created task
        self.get_task_by_id(pool, domain, task_id).await
    }

    pub async fn get_task_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
    ) -> Result<TaskResponse, anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Get task data as separate queries to avoid tuple size limits
        let task_id_str = client
            .query("SELECT id FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?
            .ok_or_else(|| anyhow::anyhow!("Task not found"))?;

        let task_type = client
            .query("SELECT task_type FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let payload = client
            .query("SELECT payload FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let status = client
            .query("SELECT status FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let priority_u8 = client
            .query("SELECT priority FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<u8>()
            .await?;

        let max_attempts = client
            .query("SELECT max_attempts FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<u32>()
            .await?;

        let attempts = client
            .query("SELECT attempts FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<u32>()
            .await?;

        let scheduled_at_str = client
            .query("SELECT scheduled_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let started_at_str = client
            .query("SELECT started_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let completed_at_str = client
            .query("SELECT completed_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let failed_at_str = client
            .query("SELECT failed_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let error_message = client
            .query("SELECT error_message FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let created_by_str = client
            .query("SELECT created_by FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let task_domain = client
            .query("SELECT domain FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM tasks WHERE id = ? AND domain = ?")
            .bind(task_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        // Parse values
        let parsed_task_id = Uuid::parse_str(&task_id_str)?;
        let priority = self.parse_task_priority(priority_u8);
        let scheduled_at = DateTime::parse_from_rfc3339(&scheduled_at_str)?.with_timezone(&Utc);
        let started_at = started_at_str
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let completed_at = completed_at_str
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let failed_at = failed_at_str
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let created_by = created_by_str
            .as_ref()
            .and_then(|s| Uuid::parse_str(s).ok());
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        let payload_json: serde_json::Value = serde_json::from_str(&payload)?;

        Ok(TaskResponse {
            id: parsed_task_id,
            task_type,
            payload: payload_json,
            status: self.parse_task_status(&status),
            priority,
            max_attempts,
            attempts,
            scheduled_at,
            started_at,
            completed_at,
            failed_at,
            error_message,
            created_by,
            domain: task_domain,
            created_at,
            updated_at,
        })
    }

    pub async fn get_pending_tasks(
        &self,
        pool: &DatabasePool,
        domain: &str,
        limit: Option<u32>,
    ) -> Result<Vec<TaskResponse>, anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let limit = limit.unwrap_or(50);

        // Get task IDs first
        let task_ids_str = client
            .query(
                r#"
                SELECT id FROM tasks
                WHERE domain = ? AND status IN ('pending', 'retry')
                      AND scheduled_at <= now() AND attempts < max_attempts
                ORDER BY priority DESC, scheduled_at ASC
                LIMIT ?
                "#,
            )
            .bind(domain)
            .bind(limit)
            .fetch_all::<String>()
            .await?;

        let mut tasks = Vec::new();
        for task_id_str in task_ids_str {
            let task_id = Uuid::parse_str(&task_id_str)?;
            if let Ok(task) = self.get_task_by_id(pool, domain, task_id).await {
                tasks.push(task);
            }
        }

        Ok(tasks)
    }

    pub async fn update_task_status(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
        status: TaskStatus,
        error_message: Option<String>,
    ) -> Result<(), anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let now = Utc::now();

        let mut query = "UPDATE tasks SET status = ?, updated_at = ?".to_string();
        let mut binds: Vec<String> = vec![serde_json::to_string(&status)?, now.to_string()];

        match status {
            TaskStatus::Running => {
                query.push_str(", started_at = ?, attempts = attempts + 1");
                binds.push(now.to_string());
            }
            TaskStatus::Completed => {
                query.push_str(", completed_at = ?");
                binds.push(now.to_string());
            }
            TaskStatus::Failed => {
                query.push_str(", failed_at = ?");
                binds.push(now.to_string());
                if error_message.is_some() {
                    query.push_str(", error_message = ?");
                    binds.push(error_message.clone().unwrap());
                }
            }
            _ => {}
        }

        query.push_str(" WHERE id = ? AND domain = ?");
        binds.push(task_id.to_string());
        binds.push(domain.to_string());

        let mut clickhouse_query = client.query(&query);
        for bind in binds {
            clickhouse_query = clickhouse_query.bind(bind);
        }
        clickhouse_query.execute().await?;

        Ok(())
    }

    pub async fn execute_task(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task: TaskResponse,
        worker_id: &str,
    ) -> Result<(), anyhow::Error> {
        let registry = self.task_registry.read().await;
        let handler = registry.get(&task.task_type);

        if let Some(handler) = handler {
            // Update task status to running
            self.update_task_status(pool, domain, task.id, TaskStatus::Running, None)
                .await?;

            // Record execution start
            self.record_task_execution(
                pool,
                domain,
                task.id,
                worker_id,
                TaskExecutionStatus::Started,
            )
            .await?;

            let start_time = std::time::Instant::now();

            // Execute the task
            match handler.execute(pool, domain, &task).await {
                Ok(_) => {
                    // Task completed successfully
                    self.update_task_status(pool, domain, task.id, TaskStatus::Completed, None)
                        .await?;
                    self.record_task_execution_completion(
                        pool,
                        domain,
                        task.id,
                        worker_id,
                        true,
                        None,
                        start_time.elapsed(),
                    )
                    .await?;
                }
                Err(e) => {
                    // Task failed - convert error to string immediately
                    let error_msg = format!("{:?}", e);

                    // Log the error first
                    tracing::error!("Task {} failed: {}", task.id, error_msg);

                    // Update task status
                    if let Err(status_err) = self
                        .update_task_status(
                            pool,
                            domain,
                            task.id,
                            TaskStatus::Failed,
                            Some(error_msg.clone()),
                        )
                        .await
                    {
                        tracing::error!("Failed to update task status: {:?}", status_err);
                    }

                    // Record execution completion
                    if let Err(record_err) = self
                        .record_task_execution_completion(
                            pool,
                            domain,
                            task.id,
                            worker_id,
                            false,
                            Some(error_msg),
                            start_time.elapsed(),
                        )
                        .await
                    {
                        tracing::error!("Failed to record task execution: {:?}", record_err);
                    }

                    // Check if we should retry
                    if task.attempts + 1 < task.max_attempts {
                        if let Err(retry_err) = self.schedule_retry(pool, domain, task.id).await {
                            tracing::error!("Failed to schedule retry: {:?}", retry_err);
                        }
                    }
                }
            }
        } else {
            // No handler found for this task type
            let error_msg = format!("No handler found for task type: {}", task.task_type);
            self.update_task_status(
                pool,
                domain,
                task.id,
                TaskStatus::Failed,
                Some(error_msg.clone()),
            )
            .await?;
            self.record_task_execution_completion(
                pool,
                domain,
                task.id,
                worker_id,
                false,
                Some(error_msg),
                std::time::Duration::from_millis(0),
            )
            .await?;
        }

        Ok(())
    }

    pub async fn cancel_task(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        self.update_task_status(pool, domain, task_id, TaskStatus::Cancelled, None)
            .await?;
        Ok(())
    }

    pub async fn cleanup_old_tasks(
        &self,
        pool: &DatabasePool,
        domain: &str,
        older_than_days: u32,
    ) -> Result<u64, anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let cutoff_date = Utc::now() - Duration::days(older_than_days as i64);

        // Delete completed tasks older than the cutoff
        let result = client
            .query("DELETE FROM tasks WHERE domain = ? AND status IN ('completed', 'failed', 'cancelled') AND updated_at < ?")
            .bind(domain)
            .bind(cutoff_date)
            .execute()
            .await?;

        // Also cleanup old task executions
        client
            .query("DELETE FROM task_executions WHERE created_at < ?")
            .bind(cutoff_date)
            .execute()
            .await?;

        Ok(0) // ClickHouse doesn't return affected rows count
    }

    async fn schedule_retry(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
    ) -> Result<(), anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Schedule retry with exponential backoff (attempts * 5 minutes)
        let task = self.get_task_by_id(pool, domain, task_id).await?;
        let retry_delay = Duration::minutes((task.attempts + 1) as i64 * 5);
        let retry_at = Utc::now() + retry_delay;

        client
            .query("UPDATE tasks SET status = ?, scheduled_at = ?, updated_at = ? WHERE id = ? AND domain = ?")
            .bind("retry")
            .bind(retry_at)
            .bind(Utc::now())
            .bind(task_id)
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    async fn record_task_execution(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
        worker_id: &str,
        status: TaskExecutionStatus,
    ) -> Result<(), anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let execution_id = Uuid::new_v4();
        let now = Utc::now();

        client
            .query("INSERT INTO task_executions (id, task_id, worker_id, status, started_at, created_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(execution_id)
            .bind(task_id)
            .bind(worker_id)
            .bind(serde_json::to_string(&status)?)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        Ok(())
    }

    async fn record_task_execution_completion(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task_id: Uuid,
        worker_id: &str,
        success: bool,
        error_message: Option<String>,
        duration: std::time::Duration,
    ) -> Result<(), anyhow::Error> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let now = Utc::now();
        let status = if success {
            TaskExecutionStatus::Completed
        } else {
            TaskExecutionStatus::Failed
        };

        client
            .query(
                r#"
                UPDATE task_executions
                SET status = ?, completed_at = ?, failed_at = ?, error_message = ?, duration_ms = ?
                WHERE task_id = ? AND worker_id = ? AND completed_at IS NULL
                "#,
            )
            .bind(serde_json::to_string(&status)?)
            .bind(if success { Some(now) } else { None })
            .bind(if !success { Some(now) } else { None })
            .bind(error_message)
            .bind(duration.as_millis() as u64)
            .bind(task_id)
            .bind(worker_id)
            .execute()
            .await?;

        Ok(())
    }

    async fn get_client_by_domain(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> Result<Client, anyhow::Error> {
        use crate::database::get_client_by_domain;
        get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No database connection found for domain: {}", domain))
    }

    fn parse_task_status(&self, status: &str) -> TaskStatus {
        match status {
            "pending" => TaskStatus::Pending,
            "running" => TaskStatus::Running,
            "completed" => TaskStatus::Completed,
            "failed" => TaskStatus::Failed,
            "cancelled" => TaskStatus::Cancelled,
            "retry" => TaskStatus::Retry,
            _ => TaskStatus::Pending,
        }
    }

    fn parse_task_priority(&self, priority: u8) -> TaskPriority {
        match priority {
            1 => TaskPriority::Low,
            2 => TaskPriority::Normal,
            3 => TaskPriority::High,
            4 => TaskPriority::Critical,
            _ => TaskPriority::Normal,
        }
    }
}

#[async_trait::async_trait]
pub trait TaskHandler: Send + Sync {
    async fn execute(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task: &TaskResponse,
    ) -> Result<(), anyhow::Error>;
}
