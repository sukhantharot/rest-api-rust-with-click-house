use clickhouse::Client;

pub async fn run_client_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Create users table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id String,
                username String,
                email String,
                password_hash String,
                first_name Nullable(String),
                last_name Nullable(String),
                is_active Bool DEFAULT true,
                is_verified Bool DEFAULT false,
                role_id Nullable(String),
                last_login_at Nullable(DateTime),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create roles table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS roles (
                id String,
                name String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                domain String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (domain, name)
            "#,
        )
        .execute()
        .await?;

    // Create permissions table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS permissions (
                id String,
                name String,
                resource String,
                action String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                domain String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (domain, resource, action)
            "#,
        )
        .execute()
        .await?;

    // Create role_permissions table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS role_permissions (
                id String,
                role_id String,
                permission_id String,
                domain String,
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (domain, role_id)
            "#,
        )
        .execute()
        .await?;

    // Create blog_categories table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_categories (
                id String,
                name String,
                slug String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create tags table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS tags (
                id String,
                name String,
                slug String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create blogs table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blogs (
                id String,
                title String,
                slug String,
                content String,
                excerpt Nullable(String),
                author_id String,
                status String,
                published_at Nullable(DateTime),
                meta_title Nullable(String),
                meta_description Nullable(String),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create blog_categories mapping table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_categories (
                id String,
                blog_id String,
                category_id String,
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (blog_id, category_id)
            "#,
        )
        .execute()
        .await?;

    // Create blog_tags mapping table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_tags (
                id String,
                blog_id String,
                tag_id String,
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (blog_id, tag_id)
            "#,
        )
        .execute()
        .await?;

    // Create auth_tracking table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS auth_tracking (
                id String,
                user_id Nullable(String),
                action String,
                ip_address Nullable(String),
                user_agent Nullable(String),
                success Bool,
                error_message Nullable(String),
                session_id Nullable(String),
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY created_at
            "#,
        )
        .execute()
        .await?;

    // Create blog_tracking table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_tracking (
                id String,
                blog_id String,
                user_id Nullable(String),
                action String,
                ip_address Nullable(String),
                user_agent Nullable(String),
                referrer Nullable(String),
                session_id Nullable(String),
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY created_at
            "#,
        )
        .execute()
        .await?;

    // Create tasks table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
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
            ) ENGINE = MergeTree()
            ORDER BY (status, priority, scheduled_at)
            "#,
        )
        .execute()
        .await?;

    // Create task_executions table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS task_executions (
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
            ) ENGINE = MergeTree()
            ORDER BY created_at
            "#,
        )
        .execute()
        .await?;

    println!("Client database migrations completed successfully");
    Ok(())
}
