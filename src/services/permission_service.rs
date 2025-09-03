use crate::database::DatabasePool;
use crate::models::role::*;
use chrono::{DateTime, Utc};
use clickhouse::Client;
use uuid::Uuid;

pub struct PermissionService;

impl PermissionService {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_permission(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreatePermissionRequest,
    ) -> Result<PermissionResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if permission name already exists
        let existing_count = client
            .query("SELECT count() FROM permissions WHERE name = ? AND domain = ?")
            .bind(&request.name)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if existing_count > 0 {
            return Err("Permission with this name already exists".into());
        }

        let permission_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert permission
        client
            .query(
                r#"
                INSERT INTO permissions (
                    id, name, resource, action, description, is_active, domain, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(permission_id)
            .bind(&request.name)
            .bind(&request.resource)
            .bind(&request.action)
            .bind(&request.description)
            .bind(true)
            .bind(domain)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        // Get the created permission
        self.get_permission_by_id(pool, domain, permission_id).await
    }

    pub async fn get_permission_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        permission_id: Uuid,
    ) -> Result<PermissionResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Get permission data as separate queries
        let permission_id_str = client
            .query("SELECT id FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?
            .ok_or("Permission not found")?;

        let name = client
            .query("SELECT name FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let resource = client
            .query("SELECT resource FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let action = client
            .query("SELECT action FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let description = client
            .query("SELECT description FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let is_active = client
            .query("SELECT is_active FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        // Parse values
        let parsed_permission_id = Uuid::parse_str(&permission_id_str)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        // Get role count for this permission
        let role_count = client
            .query("SELECT count() FROM role_permissions WHERE permission_id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await? as u32;

        Ok(PermissionResponse {
            id: parsed_permission_id,
            name,
            resource,
            action,
            description,
            is_active,
            role_count,
            created_at,
            updated_at,
        })
    }

    pub async fn get_permissions(
        &self,
        pool: &DatabasePool,
        domain: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<PermissionResponse>, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        // Get permission IDs first
        let permission_ids_str = client
            .query("SELECT id FROM permissions WHERE domain = ? ORDER BY created_at DESC LIMIT ? OFFSET ?")
            .bind(domain)
            .bind(limit)
            .bind(offset)
            .fetch_all::<String>()
            .await?;

        let mut permissions = Vec::new();
        for permission_id_str in permission_ids_str {
            let permission_id = Uuid::parse_str(&permission_id_str)?;
            if let Ok(permission) = self.get_permission_by_id(pool, domain, permission_id).await {
                permissions.push(permission);
            }
        }

        Ok(permissions)
    }

    pub async fn update_permission(
        &self,
        pool: &DatabasePool,
        domain: &str,
        permission_id: Uuid,
        request: UpdatePermissionRequest,
    ) -> Result<PermissionResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let now = Utc::now();

        // Build dynamic update query
        let mut query = "UPDATE permissions SET updated_at = ?".to_string();
        let mut binds: Vec<String> = vec![now.to_string()];

        if let Some(name) = &request.name {
            query.push_str(", name = ?");
            binds.push(name.clone());
        }
        if let Some(resource) = &request.resource {
            query.push_str(", resource = ?");
            binds.push(resource.clone());
        }
        if let Some(action) = &request.action {
            query.push_str(", action = ?");
            binds.push(action.clone());
        }
        if let Some(description) = &request.description {
            query.push_str(", description = ?");
            binds.push(description.clone());
        }
        if let Some(is_active) = request.is_active {
            query.push_str(", is_active = ?");
            binds.push(is_active.to_string());
        }

        query.push_str(" WHERE id = ? AND domain = ?");
        binds.push(permission_id.to_string());
        binds.push(domain.to_string());

        let mut clickhouse_query = client.query(&query).bind(now);
        for bind in binds {
            clickhouse_query = clickhouse_query.bind(bind);
        }
        clickhouse_query.execute().await?;

        // Get updated permission
        self.get_permission_by_id(pool, domain, permission_id).await
    }

    pub async fn delete_permission(
        &self,
        pool: &DatabasePool,
        domain: &str,
        permission_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Remove all role permission assignments first
        client
            .query("DELETE FROM role_permissions WHERE permission_id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .execute()
            .await?;

        // Delete the permission
        client
            .query("DELETE FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn initialize_builtin_permissions(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let builtin_permissions = vec![
            // User management
            (
                built_in_permissions::USER_CREATE,
                "user",
                "create",
                "Create users",
            ),
            (
                built_in_permissions::USER_READ,
                "user",
                "read",
                "Read users",
            ),
            (
                built_in_permissions::USER_UPDATE,
                "user",
                "update",
                "Update users",
            ),
            (
                built_in_permissions::USER_DELETE,
                "user",
                "delete",
                "Delete users",
            ),
            // Role management
            (
                built_in_permissions::ROLE_CREATE,
                "role",
                "create",
                "Create roles",
            ),
            (
                built_in_permissions::ROLE_READ,
                "role",
                "read",
                "Read roles",
            ),
            (
                built_in_permissions::ROLE_UPDATE,
                "role",
                "update",
                "Update roles",
            ),
            (
                built_in_permissions::ROLE_DELETE,
                "role",
                "delete",
                "Delete roles",
            ),
            (
                built_in_permissions::ROLE_ASSIGN,
                "role",
                "assign",
                "Assign roles to users",
            ),
            // Permission management
            (
                built_in_permissions::PERMISSION_CREATE,
                "permission",
                "create",
                "Create permissions",
            ),
            (
                built_in_permissions::PERMISSION_READ,
                "permission",
                "read",
                "Read permissions",
            ),
            (
                built_in_permissions::PERMISSION_UPDATE,
                "permission",
                "update",
                "Update permissions",
            ),
            (
                built_in_permissions::PERMISSION_DELETE,
                "permission",
                "delete",
                "Delete permissions",
            ),
            (
                built_in_permissions::PERMISSION_ASSIGN,
                "permission",
                "assign",
                "Assign permissions to roles",
            ),
            // Blog management
            (
                built_in_permissions::BLOG_CREATE,
                "blog",
                "create",
                "Create blog posts",
            ),
            (
                built_in_permissions::BLOG_READ,
                "blog",
                "read",
                "Read blog posts",
            ),
            (
                built_in_permissions::BLOG_UPDATE,
                "blog",
                "update",
                "Update blog posts",
            ),
            (
                built_in_permissions::BLOG_DELETE,
                "blog",
                "delete",
                "Delete blog posts",
            ),
            (
                built_in_permissions::BLOG_PUBLISH,
                "blog",
                "publish",
                "Publish blog posts",
            ),
            // Task management
            (
                built_in_permissions::TASK_CREATE,
                "task",
                "create",
                "Create tasks",
            ),
            (
                built_in_permissions::TASK_READ,
                "task",
                "read",
                "Read tasks",
            ),
            (
                built_in_permissions::TASK_UPDATE,
                "task",
                "update",
                "Update tasks",
            ),
            (
                built_in_permissions::TASK_DELETE,
                "task",
                "delete",
                "Delete tasks",
            ),
            // System
            (
                built_in_permissions::SYSTEM_ADMIN,
                "system",
                "admin",
                "System administration",
            ),
            (
                built_in_permissions::SYSTEM_MONITOR,
                "system",
                "monitor",
                "System monitoring",
            ),
        ];

        for (name, resource, action, description) in builtin_permissions {
            let request = CreatePermissionRequest {
                name: name.to_string(),
                resource: resource.to_string(),
                action: action.to_string(),
                description: Some(description.to_string()),
            };

            // Try to create, ignore if already exists
            let _ = self.create_permission(pool, domain, request).await;
        }

        Ok(())
    }

    async fn get_client_by_domain(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> Result<Client, Box<dyn std::error::Error>> {
        use crate::database::get_client_by_domain;
        get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| format!("No database connection found for domain: {}", domain).into())
    }
}
