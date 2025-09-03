use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub permissions: Vec<Permission>,
    pub user_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct Permission {
    pub id: Uuid,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePermissionRequest {
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePermissionRequest {
    pub name: Option<String>,
    pub resource: Option<String>,
    pub action: Option<String>,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionResponse {
    pub id: Uuid,
    pub name: String,
    pub resource: String,
    pub action: String,
    pub description: Option<String>,
    pub is_active: bool,
    pub role_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct RolePermission {
    pub id: Uuid,
    pub role_id: Uuid,
    pub permission_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignPermissionRequest {
    pub permission_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssignRoleRequest {
    pub user_id: Uuid,
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemovePermissionRequest {
    pub permission_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BulkPermissionAssignmentRequest {
    pub permission_ids: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserRoleInfo {
    pub user_id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Option<Role>,
    pub permissions: Vec<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoleStats {
    pub total_roles: u64,
    pub active_roles: u64,
    pub total_permissions: u64,
    pub active_permissions: u64,
    pub total_users_with_roles: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckRequest {
    pub user_id: Uuid,
    pub resource: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionCheckResponse {
    pub has_permission: bool,
    pub user_role: Option<String>,
    pub required_permission: String,
}

// Built-in roles
pub mod built_in_roles {
    pub const ADMIN: &str = "admin";
    pub const MODERATOR: &str = "moderator";
    pub const USER: &str = "user";
    pub const GUEST: &str = "guest";
}

// Built-in permissions
pub mod built_in_permissions {
    // User management
    pub const USER_CREATE: &str = "user.create";
    pub const USER_READ: &str = "user.read";
    pub const USER_UPDATE: &str = "user.update";
    pub const USER_DELETE: &str = "user.delete";

    // Role management
    pub const ROLE_CREATE: &str = "role.create";
    pub const ROLE_READ: &str = "role.read";
    pub const ROLE_UPDATE: &str = "role.update";
    pub const ROLE_DELETE: &str = "role.delete";
    pub const ROLE_ASSIGN: &str = "role.assign";

    // Permission management
    pub const PERMISSION_CREATE: &str = "permission.create";
    pub const PERMISSION_READ: &str = "permission.read";
    pub const PERMISSION_UPDATE: &str = "permission.update";
    pub const PERMISSION_DELETE: &str = "permission.delete";
    pub const PERMISSION_ASSIGN: &str = "permission.assign";

    // Blog management
    pub const BLOG_CREATE: &str = "blog.create";
    pub const BLOG_READ: &str = "blog.read";
    pub const BLOG_UPDATE: &str = "blog.update";
    pub const BLOG_DELETE: &str = "blog.delete";
    pub const BLOG_PUBLISH: &str = "blog.publish";

    // Task management
    pub const TASK_CREATE: &str = "task.create";
    pub const TASK_READ: &str = "task.read";
    pub const TASK_UPDATE: &str = "task.update";
    pub const TASK_DELETE: &str = "task.delete";

    // System
    pub const SYSTEM_ADMIN: &str = "system.admin";
    pub const SYSTEM_MONITOR: &str = "system.monitor";
}
