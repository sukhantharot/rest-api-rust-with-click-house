use crate::database::DatabasePool;
use crate::models::role::*;
use chrono::{DateTime, Utc};
use clickhouse::Client;
use uuid::Uuid;

pub struct RoleService;

impl RoleService {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateRoleRequest,
    ) -> Result<RoleResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if role name already exists
        let existing_count = client
            .query("SELECT count() FROM roles WHERE name = ? AND domain = ?")
            .bind(&request.name)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if existing_count > 0 {
            return Err("Role with this name already exists".into());
        }

        let role_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert role
        client
            .query(
                r#"
                INSERT INTO roles (
                    id, name, description, is_active, domain, created_at, updated_at
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(role_id)
            .bind(&request.name)
            .bind(&request.description)
            .bind(true)
            .bind(domain)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        // Get the created role with empty permissions
        let role_id_str = client
            .query("SELECT id FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let name = client
            .query("SELECT name FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let description = client
            .query("SELECT description FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let is_active = client
            .query("SELECT is_active FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let parsed_role_id = Uuid::parse_str(&role_id_str)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        Ok(RoleResponse {
            id: parsed_role_id,
            name,
            description,
            is_active,
            permissions: Vec::new(),
            user_count: 0,
            created_at,
            updated_at,
        })
    }

    pub async fn get_role_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
    ) -> Result<RoleResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Get role data as separate queries
        let role_id_str = client
            .query("SELECT id FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?
            .ok_or("Role not found")?;

        let name = client
            .query("SELECT name FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let description = client
            .query("SELECT description FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let is_active = client
            .query("SELECT is_active FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        // Parse values
        let parsed_role_id = Uuid::parse_str(&role_id_str)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        // Get permissions for this role
        let permissions = self.get_role_permissions(&client, domain, role_id).await?;

        // Get user count for this role
        let user_count = client
            .query("SELECT count() FROM users WHERE role_id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await? as u32;

        Ok(RoleResponse {
            id: parsed_role_id,
            name,
            description,
            is_active,
            permissions,
            user_count,
            created_at,
            updated_at,
        })
    }

    pub async fn initialize_builtin_roles(
        &self,
        pool: &DatabasePool,
        domain: &str,
        permission_service: &super::permission_service::PermissionService,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Initialize built-in permissions first
        permission_service
            .initialize_builtin_permissions(pool, domain)
            .await?;

        let builtin_roles = vec![
            (
                built_in_roles::ADMIN,
                "Administrator with full system access",
                vec![
                    built_in_permissions::USER_CREATE,
                    built_in_permissions::USER_READ,
                    built_in_permissions::USER_UPDATE,
                    built_in_permissions::USER_DELETE,
                    built_in_permissions::ROLE_CREATE,
                    built_in_permissions::ROLE_READ,
                    built_in_permissions::ROLE_UPDATE,
                    built_in_permissions::ROLE_DELETE,
                    built_in_permissions::ROLE_ASSIGN,
                    built_in_permissions::PERMISSION_ASSIGN,
                    built_in_permissions::BLOG_CREATE,
                    built_in_permissions::BLOG_READ,
                    built_in_permissions::BLOG_UPDATE,
                    built_in_permissions::BLOG_DELETE,
                    built_in_permissions::BLOG_PUBLISH,
                    built_in_permissions::TASK_CREATE,
                    built_in_permissions::TASK_READ,
                    built_in_permissions::TASK_UPDATE,
                    built_in_permissions::TASK_DELETE,
                    built_in_permissions::SYSTEM_ADMIN,
                    built_in_permissions::SYSTEM_MONITOR,
                ],
            ),
            (
                built_in_roles::MODERATOR,
                "Content moderator with blog management access",
                vec![
                    built_in_permissions::USER_READ,
                    built_in_permissions::BLOG_CREATE,
                    built_in_permissions::BLOG_READ,
                    built_in_permissions::BLOG_UPDATE,
                    built_in_permissions::BLOG_PUBLISH,
                    built_in_permissions::TASK_READ,
                ],
            ),
            (
                built_in_roles::USER,
                "Regular user with basic access",
                vec![
                    built_in_permissions::USER_READ,
                    built_in_permissions::BLOG_READ,
                    built_in_permissions::TASK_READ,
                ],
            ),
            (
                built_in_roles::GUEST,
                "Guest user with minimal access",
                vec![built_in_permissions::BLOG_READ],
            ),
        ];

        for (role_name, description, permission_names) in builtin_roles {
            // Create the role
            let role_request = CreateRoleRequest {
                name: role_name.to_string(),
                description: Some(description.to_string()),
            };

            let role = match self.create_role(pool, domain, role_request).await {
                Ok(r) => r,
                Err(_) => continue, // Role might already exist
            };

            // Assign permissions to the role
            for perm_name in permission_names {
                if let Ok(perm_id) = self
                    .get_permission_id_by_name(pool, domain, perm_name)
                    .await
                {
                    let _ = self
                        .assign_permission_to_role(pool, domain, role.id, perm_id)
                        .await;
                }
            }
        }

        Ok(())
    }

    async fn get_permission_id_by_name(
        &self,
        pool: &DatabasePool,
        domain: &str,
        permission_name: &str,
    ) -> Result<Uuid, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let permission_id_str = client
            .query("SELECT id FROM permissions WHERE name = ? AND domain = ?")
            .bind(permission_name)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let permission_id = Uuid::parse_str(&permission_id_str)?;

        Ok(permission_id)
    }

    async fn get_role_permissions(
        &self,
        client: &Client,
        domain: &str,
        role_id: Uuid,
    ) -> Result<Vec<Permission>, Box<dyn std::error::Error>> {
        let permission_ids_str = client
            .query("SELECT permission_id FROM role_permissions WHERE role_id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_all::<String>()
            .await?;

        let mut permissions = Vec::new();
        for permission_id_str in permission_ids_str {
            let permission_id = Uuid::parse_str(&permission_id_str)?;
            if let Some(perm) = self
                .get_permission_by_id_simple(client, domain, permission_id)
                .await?
            {
                permissions.push(perm);
            }
        }

        Ok(permissions)
    }

    async fn get_permission_by_id_simple(
        &self,
        client: &Client,
        domain: &str,
        permission_id: Uuid,
    ) -> Result<Option<Permission>, Box<dyn std::error::Error>> {
        // Get permission data as separate queries
        let permission_id_str = client
            .query("SELECT id FROM permissions WHERE id = ? AND domain = ?")
            .bind(permission_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        if permission_id_str.is_none() {
            return Ok(None);
        }

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

        let parsed_permission_id = Uuid::parse_str(&permission_id_str.unwrap())?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        Ok(Some(Permission {
            id: parsed_permission_id,
            name,
            resource,
            action,
            description,
            is_active,
            created_at,
            updated_at,
        }))
    }

    pub async fn get_roles(
        &self,
        pool: &DatabasePool,
        domain: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<RoleResponse>, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        // Get role IDs first
        let role_ids_str = client
            .query(
                "SELECT id FROM roles WHERE domain = ? ORDER BY created_at DESC LIMIT ? OFFSET ?",
            )
            .bind(domain)
            .bind(limit as u64)
            .bind(offset as u64)
            .fetch_all::<String>()
            .await?;

        let mut roles = Vec::new();
        for role_id_str in role_ids_str {
            let role_id = Uuid::parse_str(&role_id_str)?;
            if let Ok(role) = self.get_role_by_id(pool, domain, role_id).await {
                roles.push(role);
            }
        }

        Ok(roles)
    }

    pub async fn update_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
        payload: crate::models::role::UpdateRoleRequest,
    ) -> Result<RoleResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if role exists
        let role_exists = client
            .query("SELECT count() FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if role_exists == 0 {
            return Err("Role not found".into());
        }

        // Update role
        let now = Utc::now();
        let mut query = "UPDATE roles SET updated_at = ?".to_string();
        let mut binds: Vec<String> = vec![now.to_rfc3339()];

        if let Some(name) = &payload.name {
            query.push_str(", name = ?");
            binds.push(name.clone());
        }

        if let Some(description) = &payload.description {
            query.push_str(", description = ?");
            binds.push(description.clone());
        }

        query.push_str(" WHERE id = ? AND domain = ?");
        binds.push(role_id.to_string());
        binds.push(domain.to_string());

        let mut sql = client.query(&query);
        for bind in binds {
            sql = sql.bind(bind);
        }
        sql.execute().await?;

        // Return updated role
        self.get_role_by_id(pool, domain, role_id).await
    }

    pub async fn delete_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if role is assigned to any users
        let user_count = client
            .query("SELECT count() FROM users WHERE role_id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if user_count > 0 {
            return Err("Cannot delete role assigned to users".into());
        }

        // Delete role permissions first
        client
            .query("DELETE FROM role_permissions WHERE role_id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .execute()
            .await?;

        // Delete role
        client
            .query("DELETE FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn remove_permission_from_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
        permission_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        client
            .query("DELETE FROM role_permissions WHERE role_id = ? AND permission_id = ? AND domain = ?")
            .bind(role_id)
            .bind(permission_id)
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn bulk_assign_permissions_to_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
        permission_ids: Vec<Uuid>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        for permission_id in permission_ids {
            // Skip if already assigned
            let existing_count = client
                .query("SELECT count() FROM role_permissions WHERE role_id = ? AND permission_id = ? AND domain = ?")
                .bind(role_id)
                .bind(permission_id)
                .bind(domain)
                .fetch_one::<u64>()
                .await?;

            if existing_count == 0 {
                let assignment_id = Uuid::new_v4();
                let now = Utc::now();

                client
                    .query("INSERT INTO role_permissions (id, role_id, permission_id, domain, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(assignment_id.to_string())
                    .bind(role_id.to_string())
                    .bind(permission_id.to_string())
                    .bind(domain)
                    .bind(now.to_rfc3339())
                    .bind(now.to_rfc3339())
                    .execute()
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn get_role_stats(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> Result<crate::models::role::RoleStats, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let total_roles = client
            .query("SELECT count() FROM roles WHERE domain = ?")
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        let total_permissions = client
            .query("SELECT count() FROM permissions WHERE domain = ?")
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        let total_assignments = client
            .query("SELECT count() FROM role_permissions WHERE domain = ?")
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        Ok(crate::models::role::RoleStats {
            total_roles,
            active_roles: total_roles, // For now, assume all roles are active
            total_permissions,
            active_permissions: total_permissions, // For now, assume all permissions are active
            total_users_with_roles: total_assignments,
        })
    }

    pub async fn get_user_role_info(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<crate::models::role::UserRoleInfo, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let role_id_str = client
            .query("SELECT role_id FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        // Get user info
        let user_info = client
            .query("SELECT username, email FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<(String, String)>()
            .await?;

        let (role, permissions) = if let Some(rid_str) = role_id_str {
            let role_id = Uuid::parse_str(&rid_str)?;
            let role_response = self.get_role_by_id(pool, domain, role_id).await?;
            let role = Some(crate::models::role::Role {
                id: role_response.id,
                name: role_response.name,
                description: role_response.description,
                is_active: role_response.is_active,
                created_at: role_response.created_at,
                updated_at: role_response.updated_at,
            });
            let permissions = role_response.permissions;
            (role, permissions)
        } else {
            (None, vec![])
        };

        Ok(crate::models::role::UserRoleInfo {
            user_id,
            username: user_info.0,
            email: user_info.1,
            role,
            permissions,
        })
    }

    pub async fn assign_role_to_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
        role_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if user exists
        let user_exists = client
            .query("SELECT count() FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if user_exists == 0 {
            return Err("User not found".into());
        }

        // Check if role exists
        let role_exists = client
            .query("SELECT count() FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if role_exists == 0 {
            return Err("Role not found".into());
        }

        // Update user role
        let now = Utc::now();
        client
            .query("UPDATE users SET role_id = ?, updated_at = ? WHERE id = ? AND domain = ?")
            .bind(role_id.to_string())
            .bind(now.to_rfc3339())
            .bind(user_id.to_string())
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn remove_role_from_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let now = Utc::now();
        client
            .query("UPDATE users SET role_id = NULL, updated_at = ? WHERE id = ? AND domain = ?")
            .bind(now.to_rfc3339())
            .bind(user_id.to_string())
            .bind(domain)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn check_user_permission(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
        resource: &str,
        action: &str,
    ) -> Result<crate::models::role::PermissionCheckResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Get user's role
        let role_id_str = client
            .query("SELECT role_id FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let has_permission = if let Some(ref rid_str) = role_id_str {
            let role_id = Uuid::parse_str(rid_str)?;

            // Check if role has the permission
            let permission_count = client
                .query(
                    r#"
                    SELECT count() FROM role_permissions rp
                    JOIN permissions p ON rp.permission_id = p.id
                    WHERE rp.role_id = ? AND p.resource = ? AND p.action = ? AND rp.domain = ?
                "#,
                )
                .bind(role_id.to_string())
                .bind(resource)
                .bind(action)
                .bind(domain)
                .fetch_one::<u64>()
                .await?;

            permission_count > 0
        } else {
            false
        };

        let user_role = if let Some(rid_str) = role_id_str {
            let role_id = Uuid::parse_str(&rid_str)?;
            let role = self.get_role_by_id(pool, domain, role_id).await?;
            Some(role.name)
        } else {
            None
        };

        Ok(crate::models::role::PermissionCheckResponse {
            has_permission,
            user_role,
            required_permission: format!("{}:{}", resource, action),
        })
    }

    pub async fn assign_permission_to_role(
        &self,
        pool: &DatabasePool,
        domain: &str,
        role_id: Uuid,
        permission_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if assignment already exists
        let existing_count = client
            .query("SELECT count() FROM role_permissions WHERE role_id = ? AND permission_id = ? AND domain = ?")
            .bind(role_id)
            .bind(permission_id)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if existing_count > 0 {
            return Err("Permission already assigned to this role".into());
        }

        // Create assignment
        let assignment_id = Uuid::new_v4();
        let now = Utc::now();

        client
            .query("INSERT INTO role_permissions (id, role_id, permission_id, domain, created_at) VALUES (?, ?, ?, ?, ?)")
            .bind(assignment_id)
            .bind(role_id)
            .bind(permission_id)
            .bind(domain)
            .bind(now)
            .execute()
            .await?;

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
