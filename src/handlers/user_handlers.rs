use crate::database::DatabasePool;
use crate::models::user::*;
use crate::services::UserService;
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
pub struct PaginationQuery {
    pub limit: Option<u32>,
    pub offset: Option<u32>,
}

pub fn user_routes() -> Router<DatabasePool> {
    Router::new()
        .route("/users", get(get_users))
        .route("/users", post(create_user))
        .route("/users/:id", get(get_user))
        .route("/users/:id", put(update_user))
        .route("/users/:id", delete(delete_user))
        .route("/auth/login", post(login))
}

async fn create_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service.create_user(&pool, &domain, payload).await {
        Ok(user) => Ok(Json(user)),
        Err(e) => Err((StatusCode::BAD_REQUEST, e.to_string())),
    }
}

async fn get_users(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service
        .get_users(&pool, &domain, pagination.limit, pagination.offset)
        .await
    {
        Ok(users) => Ok(Json(users)),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string())),
    }
}

async fn get_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service.get_user_by_id(&pool, &domain, user_id).await {
        Ok(user) => Ok(Json(user)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "User not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn update_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<UserResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service
        .update_user(&pool, &domain, user_id, payload)
        .await
    {
        Ok(user) => Ok(Json(user)),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "User not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn delete_user(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(user_id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service.delete_user(&pool, &domain, user_id).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(e) => {
            if e.to_string().contains("not found") {
                Err((StatusCode::NOT_FOUND, "User not found".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

async fn login(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, (StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers).map_err(|e| (StatusCode::BAD_REQUEST, e))?;
    let user_service = UserService::new(
        "your-jwt-secret".to_string(), // TODO: Get from config
        24,
    );

    match user_service
        .authenticate_user(&pool, &domain, payload)
        .await
    {
        Ok(login_response) => Ok(Json(login_response)),
        Err(e) => {
            if e.to_string().contains("Invalid credentials") {
                Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))
            } else {
                Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
            }
        }
    }
}

fn extract_domain_from_headers(headers: &HeaderMap) -> Result<String, String> {
    // Extract domain from Host header or custom header
    let host = headers
        .get("host")
        .or_else(|| headers.get("x-tenant-domain"))
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| "Missing domain information".to_string())?;

    // Extract domain from host (remove port if present)
    let domain = host.split(':').next().unwrap_or(host);

    Ok(domain.to_string())
}
