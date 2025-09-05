use crate::database::DatabasePool;
use crate::models::task::*;
use crate::services::{TaskRegistry, TaskService};
use chrono::{DateTime, Duration, Utc};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration as TokioDuration};

pub struct TaskWorker {
    task_service: TaskService,
    pool: DatabasePool,
    worker_id: String,
    domain: String,
    poll_interval: TokioDuration,
    max_concurrent_tasks: usize,
    running: Arc<RwLock<bool>>,
}

impl TaskWorker {
    pub fn new(
        task_service: TaskService,
        pool: DatabasePool,
        domain: String,
        poll_interval_seconds: u64,
        max_concurrent_tasks: usize,
    ) -> Self {
        let worker_id = format!("worker-{}-{}", domain, uuid::Uuid::new_v4().simple());

        Self {
            task_service,
            pool,
            worker_id,
            domain,
            poll_interval: TokioDuration::from_secs(poll_interval_seconds),
            max_concurrent_tasks,
            running: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut running = self.running.write().await;
        if *running {
            return Err("Worker is already running".into());
        }
        *running = true;
        drop(running);

        tracing::info!(
            "Starting task worker {} for domain {}",
            self.worker_id,
            self.domain
        );

        let running_clone = Arc::clone(&self.running);
        let poll_interval = self.poll_interval;
        let worker_id = self.worker_id.clone();
        let pool = self.pool.clone();
        let domain = self.domain.clone();

        tokio::spawn(async move {
            let mut interval = time::interval(poll_interval);

            loop {
                interval.tick().await;

                // Check if we should stop
                {
                    let running = running_clone.read().await;
                    if !*running {
                        break;
                    }
                }

                // Process pending tasks
                let task_service = TaskService::new();
                match task_service
                    .get_pending_tasks(&pool, &domain, Some(10))
                    .await
                {
                    Ok(pending_tasks) => {
                        if !pending_tasks.is_empty() {
                            tracing::debug!(
                                "Found {} pending tasks for domain {}",
                                pending_tasks.len(),
                                domain
                            );

                            // Process tasks concurrently but with a limit
                            let mut handles = Vec::new();
                            let max_concurrent = 5; // Use a fixed limit for the spawned task

                            for task in pending_tasks {
                                let pool_clone = pool.clone();
                                let domain_clone = domain.clone();
                                let worker_id_clone = worker_id.clone();
                                let task_service_clone = Arc::new(TaskService::new());

                                let handle = tokio::spawn(async move {
                                    if let Err(e) = task_service_clone
                                        .execute_task(
                                            &pool_clone,
                                            &domain_clone,
                                            task,
                                            &worker_id_clone,
                                        )
                                        .await
                                    {
                                        tracing::error!("Error executing task: {:?}", e);
                                    }
                                });

                                handles.push(handle);

                                // If we've reached the max concurrent tasks limit, wait for some to complete
                                if handles.len() >= max_concurrent {
                                    // Wait for the first task to complete
                                    if let Some(handle) = handles.first_mut() {
                                        let _ = handle.await;
                                        handles.remove(0);
                                    }
                                }
                            }

                            // Wait for all remaining tasks to complete
                            for handle in handles {
                                let _ = handle.await;
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!(
                            "Error getting pending tasks for domain {}: {:?}",
                            domain,
                            e
                        );
                    }
                }
            }

            tracing::info!("Task worker {} stopped", worker_id);
        });

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut running = self.running.write().await;
        if !*running {
            return Err("Worker is not running".into());
        }
        *running = false;

        tracing::info!("Stopping task worker {}", self.worker_id);
        Ok(())
    }

    pub async fn is_running(&self) -> bool {
        let running = self.running.read().await;
        *running
    }

    async fn process_pending_tasks(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get pending tasks
        let pending_tasks = self
            .task_service
            .get_pending_tasks(
                &self.pool,
                &self.domain,
                Some(self.max_concurrent_tasks as u32),
            )
            .await?;

        if pending_tasks.is_empty() {
            return Ok(());
        }

        tracing::debug!(
            "Found {} pending tasks for domain {}",
            pending_tasks.len(),
            self.domain
        );

        // Process tasks concurrently but with a limit
        let mut handles = Vec::new();

        for task in pending_tasks {
            let pool = self.pool.clone();
            let domain = self.domain.clone();
            let worker_id = self.worker_id.clone();
            // Move task service creation outside to avoid lifetime issues
            let task_service = Arc::new(TaskService::new());
            let task_service_clone = task_service.clone();
            let handle = tokio::spawn(async move {
                if let Err(e) = task_service_clone
                    .execute_task(&pool, &domain, task, &worker_id)
                    .await
                {
                    tracing::error!("Error executing task: {}", e.to_string());
                }
            });

            handles.push(handle);

            // If we've reached the max concurrent tasks limit, wait for some to complete
            if handles.len() >= self.max_concurrent_tasks {
                // Wait for the first task to complete
                if let Some(handle) = handles.first_mut() {
                    let _ = handle.await;
                    handles.remove(0);
                }
            }
        }

        // Wait for all remaining tasks to complete
        for handle in handles {
            let _ = handle.await;
        }

        Ok(())
    }
}

// Task Scheduler - manages multiple workers across different domains
pub struct TaskScheduler {
    workers: Arc<RwLock<std::collections::HashMap<String, TaskWorker>>>,
    task_service: TaskService,
    pool: DatabasePool,
    default_poll_interval: u64,
    default_max_concurrent: usize,
}

impl TaskScheduler {
    pub fn new(task_service: TaskService, pool: DatabasePool) -> Self {
        Self {
            workers: Arc::new(RwLock::new(std::collections::HashMap::new())),
            task_service,
            pool,
            default_poll_interval: 30, // 30 seconds
            default_max_concurrent: 5,
        }
    }

    pub async fn start_worker_for_domain(
        &self,
        domain: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.start_worker_for_domain_with_config(
            domain,
            self.default_poll_interval,
            self.default_max_concurrent,
        )
        .await
    }

    pub async fn start_worker_for_domain_with_config(
        &self,
        domain: &str,
        poll_interval_seconds: u64,
        max_concurrent_tasks: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut workers = self.workers.write().await;

        if workers.contains_key(domain) {
            return Err(format!("Worker already exists for domain: {}", domain).into());
        }

        let worker = TaskWorker::new(
            TaskService::new(), // Each worker gets its own task service instance
            self.pool.clone(),
            domain.to_string(),
            poll_interval_seconds,
            max_concurrent_tasks,
        );

        worker.start().await?;
        workers.insert(domain.to_string(), worker);

        tracing::info!("Started task worker for domain: {}", domain);
        Ok(())
    }

    pub async fn stop_worker_for_domain(
        &self,
        domain: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut workers = self.workers.write().await;

        if let Some(worker) = workers.remove(domain) {
            worker.stop().await?;
            tracing::info!("Stopped task worker for domain: {}", domain);
        }

        Ok(())
    }

    pub async fn stop_all_workers(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut workers = self.workers.write().await;
        let domains: Vec<String> = workers.keys().cloned().collect();

        for domain in domains {
            if let Some(worker) = workers.remove(&domain) {
                worker.stop().await?;
                tracing::info!("Stopped task worker for domain: {}", domain);
            }
        }

        Ok(())
    }

    pub async fn get_worker_status(&self, domain: &str) -> Option<bool> {
        let workers = self.workers.read().await;
        workers
            .get(domain)
            .map(|w| futures::executor::block_on(w.is_running()))
    }

    pub async fn get_all_worker_statuses(&self) -> std::collections::HashMap<String, bool> {
        let workers = self.workers.read().await;
        let mut statuses = std::collections::HashMap::new();

        for (domain, worker) in workers.iter() {
            statuses.insert(
                domain.clone(),
                futures::executor::block_on(worker.is_running()),
            );
        }

        statuses
    }

    // Maintenance tasks
    pub async fn schedule_maintenance_tasks(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Get all domains from the base database
        let base_client = match crate::database::get_base_client(&self.pool).await {
            Some(client) => client,
            None => {
                tracing::warn!("No base database client available for maintenance tasks");
                return Ok(());
            }
        };

        // This is a simplified example - in practice you'd query all domains
        // TODO: Query actual domains from database instead of hardcoded list
        let domains: Vec<String> = vec![]; // Temporarily disabled for production

        for domain in domains {
            // Schedule daily cleanup task
            let cleanup_payload = CleanupTaskPayload {
                table_name: "auth_tracking".to_string(),
                older_than_days: 30,
                batch_size: Some(1000),
            };

            let cleanup_task = CreateTaskRequest {
                task_type: task_types::DATA_CLEANUP.to_string(),
                payload: serde_json::to_value(cleanup_payload)?,
                priority: Some(TaskPriority::Low),
                scheduled_at: Some(Utc::now() + Duration::hours(24)), // Tomorrow
                max_attempts: Some(3),
            };

            self.task_service
                .create_task(&self.pool, &domain, cleanup_task, None)
                .await?;
        }

        Ok(())
    }
}
