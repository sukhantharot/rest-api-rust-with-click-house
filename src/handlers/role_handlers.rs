use crate::database::DatabasePool;
use crate::models::role::*;
use crate::services::{PermissionService, RoleService};

#[derive(serde::Deserialize)]
pub struct PermissionListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

async fn create_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePermissionRequest>,
) -> Result<Json<PermissionResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .create_permission(&pool, &domain, payload)
        .await
    {
        Ok(permission) => Ok(Json(permission)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn get_permissions(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<PermissionListQuery>,
) -> Result<Json<Vec<PermissionResponse>>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .get_permissions(&pool, &domain, query.limit, query.offset)
        .await
    {
        Ok(permissions) => Ok(Json(permissions)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(permission_id): Path<Uuid>,
) -> Result<Json<PermissionResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .get_permission_by_id(&pool, &domain, permission_id)
        .await
    {
        Ok(permission) => Ok(Json(permission)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Permission not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn update_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(permission_id): Path<Uuid>,
    Json(payload): Json<UpdatePermissionRequest>,
) -> Result<Json<PermissionResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .update_permission(&pool, &domain, permission_id, payload)
        .await
    {
        Ok(permission) => Ok(Json(permission)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Permission not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn delete_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(permission_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .delete_permission(&pool, &domain, permission_id)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn initialize_builtin_permissions(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let permission_service = PermissionService::new();

    match permission_service
        .initialize_builtin_permissions(&pool, &domain)
        .await
    {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "Built-in permissions initialized successfully",
            "permissions": [
                "user.create", "user.read", "user.update", "user.delete",
                "role.create", "role.read", "role.update", "role.delete",
                "permission.create", "permission.read", "permission.update", "permission.delete",
                "blog.create", "blog.read", "blog.update", "blog.delete",
                "task.create", "task.read", "task.update", "task.delete",
                "system.admin", "system.monitor"
            ]
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

#[derive(Deserialize)]
pub struct RoleListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub fn role_routes() -> Router<DatabasePool> {
    Router::new()
        .route("/roles", get(get_roles))
        .route("/roles", post(create_role))
        .route("/roles/{id}", get(get_role))
        .route("/roles/{id}", put(update_role))
        .route("/roles/{id}", delete(delete_role))
        .route("/roles/{id}/permissions", get(get_role_permissions))
        .route("/roles/{id}/permissions", post(assign_permission_to_role))
        .route(
            "/roles/{id}/permissions/{permission_id}",
            delete(remove_permission_from_role),
        )
        .route(
            "/roles/{id}/permissions/bulk",
            post(bulk_assign_permissions),
        )
        // Temporarily disable builtin roles route
        // .route("/roles/initialize-builtin", post(initialize_builtin_roles))
        // Permission routes moved to permission_handlers.rs to avoid conflicts
        .route("/roles/stats", get(get_role_stats))
        .route("/users/{user_id}/role", get(get_user_role))
        .route("/users/{user_id}/role", post(assign_role_to_user))
        .route("/users/{user_id}/role", delete(remove_role_from_user))
        .route("/permissions/check", post(check_user_permission))
}

async fn create_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<CreateRoleRequest>,
) -> Result<Json<RoleResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service.create_role(&pool, &domain, payload).await {
        Ok(role) => Ok(Json(role)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn get_roles(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<RoleListQuery>,
) -> Result<Json<Vec<RoleResponse>>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .get_roles(&pool, &domain, query.limit, query.offset)
        .await
    {
        Ok(roles) => Ok(Json(roles)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
) -> Result<Json<RoleResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service.get_role_by_id(&pool, &domain, role_id).await {
        Ok(role) => Ok(Json(role)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Role not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn update_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(payload): Json<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .update_role(&pool, &domain, role_id, payload)
        .await
    {
        Ok(role) => Ok(Json(role)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Role not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn delete_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service.delete_role(&pool, &domain, role_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Role not found".to_string()))
            } else if e.to_string().contains("assigned to users") {
                Err((
                    StatusCode::CONFLICT,
                    "Cannot delete role assigned to users".to_string(),
                ))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn get_role_permissions(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
) -> Result<Json<Vec<Permission>>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service.get_role_by_id(&pool, &domain, role_id).await {
        Ok(role) => Ok(Json(role.permissions)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "Role not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn assign_permission_to_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(payload): Json<AssignPermissionRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .assign_permission_to_role(&pool, &domain, role_id, payload.permission_id)
        .await
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            if e.to_string().contains("already assigned") {
                Err((
                    StatusCode::CONFLICT,
                    "Permission already assigned to role".to_string(),
                ))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn remove_permission_from_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path((role_id, permission_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .remove_permission_from_role(&pool, &domain, role_id, permission_id)
        .await
    {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn bulk_assign_permissions(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(role_id): Path<Uuid>,
    Json(payload): Json<BulkPermissionAssignmentRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .bulk_assign_permissions_to_role(&pool, &domain, role_id, payload.permission_ids)
        .await
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn initialize_builtin_roles(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();
    let permission_service = PermissionService::new();

    match role_service
        .initialize_builtin_roles(&pool, &domain, &permission_service)
        .await
    {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "Built-in roles and permissions initialized successfully",
            "roles": [
                "admin", "moderator", "user", "guest"
            ]
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_role_stats(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<RoleStats>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service.get_role_stats(&pool, &domain).await {
        Ok(stats) => Ok(Json(stats)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_user_role(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserRoleInfo>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .get_user_role_info(&pool, &domain, user_id)
        .await
    {
        Ok(user_info) => Ok(Json(user_info)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "User not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn assign_role_to_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<AssignRoleRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .assign_role_to_user(&pool, &domain, user_id, payload.role_id)
        .await
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn remove_role_from_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .remove_role_from_user(&pool, &domain, user_id)
        .await
    {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn check_user_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<PermissionCheckRequest>,
) -> Result<Json<PermissionCheckResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let role_service = RoleService::new();

    match role_service
        .check_user_permission(
            &pool,
            &domain,
            payload.user_id,
            &payload.resource,
            &payload.action,
        )
        .await
    {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// Helper function to extract domain from headers
fn extract_domain_from_headers(headers: &HeaderMap) -> Result<String, String> {
    let host = headers
        .get("host")
        .or_else(|| headers.get("x-tenant-domain"))
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| "Missing domain information".to_string())?;

    let domain = host.split(':').next().unwrap_or(host);
    Ok(domain.to_string())
}
