use sqlx::PgPool;

pub async fn run_client_migrations(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Create users table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            username VARCHAR(255) UNIQUE NOT NULL,
            email VARCHAR(255) UNIQUE NOT NULL,
            password_hash VARCHAR(255) NOT NULL,
            first_name VARCHAR(255),
            last_name VARCHAR(255),
            is_active BOOLEAN DEFAULT true,
            is_verified BOOLEAN DEFAULT false,
            role_id UUID,
            last_login_at TIMESTAMPTZ,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create roles table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS roles (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            description TEXT,
            is_active BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create permissions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS permissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            resource VARCHAR(255) NOT NULL,
            action VARCHAR(255) NOT NULL,
            description TEXT,
            is_active BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(resource, action)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create role_permissions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS role_permissions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            role_id UUID NOT NULL REFERENCES roles(id) ON DELETE CASCADE,
            permission_id UUID NOT NULL REFERENCES permissions(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(role_id, permission_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create categories table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS categories (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            slug VARCHAR(255) UNIQUE NOT NULL,
            description TEXT,
            is_active BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create tags table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tags (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            slug VARCHAR(255) UNIQUE NOT NULL,
            description TEXT,
            is_active BOOLEAN DEFAULT true,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create blogs table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blogs (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            title VARCHAR(500) NOT NULL,
            slug VARCHAR(500) UNIQUE NOT NULL,
            content TEXT NOT NULL,
            excerpt TEXT,
            author_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            status VARCHAR(50) DEFAULT 'draft',
            published_at TIMESTAMPTZ,
            meta_title VARCHAR(500),
            meta_description TEXT,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create blog_categories mapping table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blog_categories (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            blog_id UUID NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
            category_id UUID NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(blog_id, category_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create blog_tags mapping table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blog_tags (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            blog_id UUID NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
            tag_id UUID NOT NULL REFERENCES tags(id) ON DELETE CASCADE,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            UNIQUE(blog_id, tag_id)
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create auth_tracking table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS auth_tracking (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            action VARCHAR(100) NOT NULL,
            ip_address INET,
            user_agent TEXT,
            success BOOLEAN NOT NULL,
            error_message TEXT,
            session_id VARCHAR(255),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create blog_tracking table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS blog_tracking (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            blog_id UUID NOT NULL REFERENCES blogs(id) ON DELETE CASCADE,
            user_id UUID REFERENCES users(id) ON DELETE SET NULL,
            ip_address INET,
            user_agent TEXT,
            viewed_at TIMESTAMPTZ DEFAULT NOW(),
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create tasks table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS tasks (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            task_type VARCHAR(255) NOT NULL,
            payload JSONB NOT NULL,
            status VARCHAR(50) DEFAULT 'pending',
            priority INTEGER DEFAULT 0,
            max_attempts INTEGER DEFAULT 3,
            attempts INTEGER DEFAULT 0,
            scheduled_at TIMESTAMPTZ DEFAULT NOW(),
            started_at TIMESTAMPTZ,
            completed_at TIMESTAMPTZ,
            failed_at TIMESTAMPTZ,
            error_message TEXT,
            created_by UUID REFERENCES users(id) ON DELETE SET NULL,
            created_at TIMESTAMPTZ DEFAULT NOW(),
            updated_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create task_executions table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS task_executions (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            task_id UUID NOT NULL REFERENCES tasks(id) ON DELETE CASCADE,
            worker_id VARCHAR(255) NOT NULL,
            status VARCHAR(50) NOT NULL,
            started_at TIMESTAMPTZ DEFAULT NOW(),
            completed_at TIMESTAMPTZ,
            failed_at TIMESTAMPTZ,
            error_message TEXT,
            duration_ms INTEGER,
            created_at TIMESTAMPTZ DEFAULT NOW()
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for better performance
    // User indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_username ON users(username)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_email ON users(email)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_role_id ON users(role_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_users_active ON users(is_active)")
        .execute(pool)
        .await?;

    // Role and permission indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_roles_name ON roles(name)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_permissions_resource_action ON permissions(resource, action)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_role_permissions_role_id ON role_permissions(role_id)",
    )
    .execute(pool)
    .await?;

    // Blog indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blogs_slug ON blogs(slug)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blogs_status ON blogs(status)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blogs_author_id ON blogs(author_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blogs_published_at ON blogs(published_at)")
        .execute(pool)
        .await?;

    // Category and tag indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_categories_slug ON categories(slug)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tags_slug ON tags(slug)")
        .execute(pool)
        .await?;

    // Tracking indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_auth_tracking_user_id ON auth_tracking(user_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_auth_tracking_created_at ON auth_tracking(created_at)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_blog_tracking_blog_id ON blog_tracking(blog_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_blog_tracking_viewed_at ON blog_tracking(viewed_at)",
    )
    .execute(pool)
    .await?;

    // Task indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_status_priority ON tasks(status, priority)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_tasks_scheduled_at ON tasks(scheduled_at)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_task_executions_task_id ON task_executions(task_id)",
    )
    .execute(pool)
    .await?;

    tracing::info!("✅ Client database migrations completed successfully");
    Ok(())
}
