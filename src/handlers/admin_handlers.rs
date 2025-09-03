use crate::database::DatabasePool;
use crate::services::AdminService;
use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
    routing::{delete, get, post, put},
    Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct ClientConnectQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClientConnectRequest {
    pub domain: String,
    pub database_url: String,
    pub database_name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClientConnectRequest {
    pub domain: Option<String>,
    pub database_url: Option<String>,
    pub database_name: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ClientConnectResponse {
    pub id: u64,
    pub domain: String,
    pub database_url: String,
    pub database_name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BaseUserLoginRequest {
    pub username_or_email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct BaseUserLoginResponse {
    pub token: String,
    pub expires_at: chrono::DateTime<chrono::Utc>,
    pub user: BaseUserResponse,
}

#[derive(Debug, Serialize)]
pub struct BaseUserResponse {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub role: Option<String>,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub fn admin_routes() -> Router<DatabasePool> {
    Router::new()
        // Base user authentication
        .route("/admin/auth/login", post(admin_login))
        // Client connection management
        .route("/admin/clients", get(get_client_connections))
        .route("/admin/clients", post(create_client_connection))
        .route("/admin/clients/:id", get(get_client_connection))
        .route("/admin/clients/:id", put(update_client_connection))
        .route("/admin/clients/:id", delete(delete_client_connection))
        .route("/admin/clients/:id/test", post(test_client_connection))
        .route("/admin/clients/:id/migrate", post(migrate_client_database))
}

// Base user authentication for admin operations
async fn admin_login(
    State(pool): State<DatabasePool>,
    Json(request): Json<BaseUserLoginRequest>,
) -> Result<Json<BaseUserLoginResponse>, (StatusCode, String)> {
    let admin_service = AdminService::new();

    match admin_service.authenticate_base_user(&pool, request).await {
        Ok(response) => Ok(Json(response)),
        Err(e) => Err((StatusCode::UNAUTHORIZED, e.to_string())),
    }
}

// Get all client connections (with pagination)
async fn get_client_connections(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<ClientConnectQuery>,
) -> Result<Json<Vec<ClientConnectResponse>>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service
        .get_client_connections(&pool, query.limit, query.offset, query.is_active)
        .await
    {
        Ok(connections) => Ok(Json(connections)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// Create new client connection
async fn create_client_connection(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(request): Json<CreateClientConnectRequest>,
) -> Result<Json<ClientConnectResponse>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service.create_client_connection(&pool, request).await {
        Ok(connection) => Ok(Json(connection)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// Get specific client connection
async fn get_client_connection(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<ClientConnectResponse>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service.get_client_connection(&pool, id).await {
        Ok(Some(connection)) => Ok(Json(connection)),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            "Client connection not found".to_string(),
        )),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// Update client connection
async fn update_client_connection(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(id): Path<u64>,
    Json(request): Json<UpdateClientConnectRequest>,
) -> Result<Json<ClientConnectResponse>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service
        .update_client_connection(&pool, id, request)
        .await
    {
        Ok(connection) => Ok(Json(connection)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// Delete (deactivate) client connection
async fn delete_client_connection(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<StatusCode, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service.delete_client_connection(&pool, id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// Test client database connection
async fn test_client_connection(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service.test_client_connection(&pool, id).await {
        Ok(result) => Ok(Json(serde_json::json!(result))),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

// Run migrations on client database
async fn migrate_client_database(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(id): Path<u64>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Verify base user authentication
    if let Err(e) = verify_base_auth(&headers) {
        return Err((StatusCode::UNAUTHORIZED, e));
    }

    let admin_service = AdminService::new();

    match admin_service.migrate_client_database(&pool, id).await {
        Ok(_) => Ok(Json(serde_json::json!({
            "message": "Client database migration completed successfully"
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

// Helper function to verify base user authentication
fn verify_base_auth(headers: &HeaderMap) -> Result<String, String> {
    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or("Missing Authorization header")?;

    if !auth_header.starts_with("Bearer ") {
        return Err("Invalid Authorization header format".to_string());
    }

    let token = &auth_header[7..]; // Remove "Bearer " prefix

    // TODO: Implement proper JWT verification for base users
    // This is a placeholder - in production, verify JWT signature and claims
    if token.is_empty() {
        return Err("Invalid token".to_string());
    }

    Ok(token.to_string())
}
