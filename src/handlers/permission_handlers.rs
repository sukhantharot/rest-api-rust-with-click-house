use crate::database::DatabasePool;
use crate::models::role::*;
use crate::services::PermissionService;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct PermissionListQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub fn permission_routes() -> Router<DatabasePool> {
    Router::new()
        .route("/permissions", get(get_permissions))
        .route("/permissions", post(create_permission))
        .route("/permissions/{id}", get(get_permission))
        .route("/permissions/{id}", put(update_permission))
        .route("/permissions/{id}", delete(delete_permission))
        .route(
            "/permissions/initialize-builtin",
            post(initialize_builtin_permissions),
        )
}

async fn create_permission(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<CreatePermissionRequest>,
) -> Result<Json<PermissionResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)?;
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
    let domain = extract_domain_from_headers(&headers)?;
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
    let domain = extract_domain_from_headers(&headers)?;
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
    let domain = extract_domain_from_headers(&headers)?;
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
    let domain = extract_domain_from_headers(&headers)?;
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
    let domain = extract_domain_from_headers(&headers)?;
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

fn extract_domain_from_headers(headers: &HeaderMap) -> Result<String, (StatusCode, String)> {
    let host = headers
        .get("host")
        .or_else(|| headers.get("x-tenant-domain"))
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                "Missing domain information".to_string(),
            )
        })?;

    let domain = host.split(':').next().unwrap_or(host);
    Ok(domain.to_string())
}
