# REST API with Rust, Axum, and ClickHouse - Multi-Tenant System

A comprehensive multi-tenant REST API built with Rust, featuring user management, role-based permissions, blog system, tracking, and a robust background task scheduler.

## Features

### Core Architecture
- **Multi-tenant**: Domain-based tenant isolation with separate ClickHouse databases
- **REST API**: Full CRUD operations for all modules
- **JWT Authentication**: Secure token-based authentication
- **Background Task Scheduler**: Asynchronous task processing with retry logic

### Modules
- **User Management**: Registration, authentication, user profiles
- **Role & Permissions**: Flexible RBAC system
- **Blog System**: Posts, categories, tags, and tracking
- **Activity Tracking**: User actions and system monitoring
- **Task Scheduler**: Background job processing

## Quick Start

### Prerequisites
- Rust 1.70+
- ClickHouse database
- Environment configuration

### Installation

1. Clone the repository:
```bash
git clone <repository-url>
cd rest-api-rust-with-click-house
```

2. Configure environment:
```bash
cp .env.example .env
# Edit .env with your ClickHouse connection details
```

3. Run the application:
```bash
cargo run
```

## Task Scheduler

The task scheduler provides asynchronous background job processing with the following features:

### Built-in Task Types

- **Email Sending**: `email.send` - Send emails via external services
- **Data Cleanup**: `data.cleanup` - Remove old data from tables
- **Report Generation**: `report.generate` - Generate various reports
- **Cache Invalidation**: `cache.invalidate` - Clear cached data
- **Notifications**: `notification.send` - Send push notifications

### Creating Tasks

```rust
use crate::models::task::*;
use crate::services::TaskService;

// Create an email task
let email_payload = EmailTaskPayload {
    to: "user@example.com".to_string(),
    subject: "Welcome!".to_string(),
    body: "Welcome to our platform!".to_string(),
    html: Some("<h1>Welcome!</h1>".to_string()),
};

let task_request = CreateTaskRequest {
    task_type: task_types::EMAIL_SEND.to_string(),
    payload: serde_json::to_value(email_payload)?,
    priority: Some(TaskPriority::Normal),
    scheduled_at: Some(Utc::now()),
    max_attempts: Some(3),
};

// Schedule the task
let task_service = TaskService::new();
let task = task_service
    .create_task(&pool, "example.com", task_request, None)
    .await?;
```

### API Endpoints

#### Create Task
```http
POST /tasks
Content-Type: application/json
Host: example.com

{
  "task_type": "email.send",
  "payload": {
    "to": "user@example.com",
    "subject": "Test Email",
    "body": "This is a test email"
  },
  "priority": "normal",
  "scheduled_at": "2024-01-01T00:00:00Z"
}
```

#### Get Tasks
```http
GET /tasks?limit=10&offset=0
Host: example.com
```

#### Cancel Task
```http
POST /tasks/{task_id}/cancel
Host: example.com
```

#### Schedule Maintenance Cleanup
```http
POST /tasks/maintenance/cleanup
Host: example.com
```

### Custom Task Handlers

Create custom task handlers by implementing the `TaskHandler` trait:

```rust
use crate::services::{TaskHandler, TaskRegistry};
use crate::models::task::*;
use crate::database::DatabasePool;

pub struct CustomTaskHandler;

#[async_trait::async_trait]
impl TaskHandler for CustomTaskHandler {
    async fn execute(
        &self,
        pool: &DatabasePool,
        domain: &str,
        task: &TaskResponse,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Your custom logic here
        tracing::info!("Executing custom task for domain: {}", domain);
        Ok(())
    }
}

// Register the handler
let task_service = TaskService::new();
task_service.register_handler("custom.task", CustomTaskHandler).await;
```

### Task Worker Configuration

The task scheduler automatically starts workers for each domain:

```rust
let task_scheduler = TaskScheduler::new(task_service, pool);

// Start worker for a specific domain
task_scheduler
    .start_worker_for_domain_with_config(
        "example.com",
        30, // Poll interval in seconds
        5,  // Max concurrent tasks
    )
    .await?;
```

### Database Schema

The task scheduler uses the following ClickHouse tables:

```sql
-- Tasks table
CREATE TABLE tasks (
    id String,
    task_type String,
    payload String,
    status String,
    priority UInt8,
    max_attempts UInt32 DEFAULT 3,
    attempts UInt32 DEFAULT 0,
    scheduled_at DateTime,
    started_at Nullable(DateTime),
    completed_at Nullable(DateTime),
    failed_at Nullable(DateTime),
    error_message Nullable(String),
    created_by Nullable(String),
    domain String,
    created_at DateTime DEFAULT now(),
    updated_at DateTime DEFAULT now()
) ENGINE = MergeTree() ORDER BY (status, priority, scheduled_at);

-- Task executions table
CREATE TABLE task_executions (
    id String,
    task_id String,
    worker_id String,
    status String,
    started_at DateTime,
    completed_at Nullable(DateTime),
    failed_at Nullable(DateTime),
    error_message Nullable(String),
    duration_ms Nullable(UInt64),
    created_at DateTime DEFAULT now()
) ENGINE = MergeTree() ORDER BY created_at;
```

## API Documentation

### Authentication
All API endpoints require domain-specific routing via the `Host` header or `x-tenant-domain` header.

### Error Handling
The API returns standard HTTP status codes with JSON error responses.

### Rate Limiting
Built-in rate limiting can be configured per domain.

## Development

### Running Tests
```bash
cargo test
```

### Building for Production
```bash
cargo build --release
```

### Database Migrations
Migrations run automatically on startup, creating all necessary tables.

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Submit a pull request

## License

This project is licensed under the MIT License.
