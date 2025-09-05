use crate::database::DatabasePool;
use crate::models::blog::*;
use crate::services::BlogService;
use axum::{
    Router,
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Json,
    routing::{delete, get, post, put},
};
use serde::{Deserialize, Serialize};

use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct BlogQuery {
    page: Option<u32>,
    limit: Option<u32>,
    status: Option<String>,
    category_id: Option<String>,
    tag_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlogSearchQuery {
    q: String,
    page: Option<u32>,
    limit: Option<u32>,
}

pub fn blog_routes() -> Router<DatabasePool> {
    Router::new()
        .route("/blogs", post(create_blog))
        .route("/blogs", get(get_blogs))
        .route("/blogs/search", get(search_blogs))
        .route("/blogs/{id}", get(get_blog))
        .route("/blogs/{id}", put(update_blog))
        .route("/blogs/{id}", delete(delete_blog))
        .route("/blogs/{id}/track", post(track_blog_view))
        .route("/blogs/{id}/stats", get(get_blog_stats))
        .route("/categories", post(create_category))
        .route("/categories", get(get_categories))
        .route("/tags", post(create_tag))
        .route("/tags", get(get_tags))
}

// Blog CRUD handlers
pub async fn create_blog(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(request): Json<CreateBlogRequest>,
) -> Result<Json<BlogResponse>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    // TODO: Extract user_id from JWT token in production
    let author_id = Uuid::new_v4(); // Placeholder

    let blog_service = BlogService::new();
    let blog = blog_service
        .create_blog(&pool, &domain, request, author_id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(blog))
}

pub async fn get_blogs(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<BlogQuery>,
) -> Result<Json<Vec<BlogResponse>>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let category_id = query
        .category_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());
    let tag_id = query
        .tag_id
        .as_ref()
        .and_then(|id| Uuid::parse_str(id).ok());

    let blog_service = BlogService::new();
    let blogs = blog_service
        .get_blogs(
            &pool,
            &domain,
            query.page,
            query.limit,
            query.status.as_deref(),
            category_id,
            tag_id,
        )
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(blogs))
}

pub async fn search_blogs(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Query(query): Query<BlogSearchQuery>,
) -> Result<Json<Vec<BlogResponse>>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_service = BlogService::new();

    // For now, we'll use the basic get_blogs and filter by search term
    // In production, you'd implement full-text search in ClickHouse
    let blogs = blog_service
        .get_blogs(&pool, &domain, query.page, query.limit, None, None, None)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // Filter blogs by search term (simple implementation)
    let filtered_blogs: Vec<BlogResponse> = blogs
        .into_iter()
        .filter(|blog| {
            blog.title.to_lowercase().contains(&query.q.to_lowercase())
                || blog
                    .content
                    .to_lowercase()
                    .contains(&query.q.to_lowercase())
                || blog
                    .excerpt
                    .as_ref()
                    .map(|e| e.to_lowercase().contains(&query.q.to_lowercase()))
                    .unwrap_or(false)
        })
        .collect();

    Ok(Json(filtered_blogs))
}

pub async fn get_blog(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(blog_id): Path<String>,
) -> Result<Json<BlogResponse>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_id = Uuid::parse_str(&blog_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid blog ID".to_string(),
        )
    })?;

    let blog_service = BlogService::new();
    let blog = blog_service
        .get_blog_by_id(&pool, &domain, blog_id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(blog))
}

pub async fn update_blog(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(blog_id): Path<String>,
    Json(request): Json<UpdateBlogRequest>,
) -> Result<Json<BlogResponse>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_id = Uuid::parse_str(&blog_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid blog ID".to_string(),
        )
    })?;

    let blog_service = BlogService::new();
    let blog = blog_service
        .update_blog(&pool, &domain, blog_id, request)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(blog))
}

pub async fn delete_blog(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(blog_id): Path<String>,
) -> Result<Json<()>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_id = Uuid::parse_str(&blog_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid blog ID".to_string(),
        )
    })?;

    let blog_service = BlogService::new();
    blog_service
        .delete_blog(&pool, &domain, blog_id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(()))
}

// Category handlers
pub async fn create_category(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(request): Json<CreateCategoryRequest>,
) -> Result<Json<CategoryResponse>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_service = BlogService::new();
    let category = blog_service
        .create_category(&pool, &domain, request)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(category))
}

pub async fn get_categories(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<Vec<CategoryResponse>>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_service = BlogService::new();
    let categories = blog_service
        .get_categories(&pool, &domain)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(categories))
}

// Tag handlers
pub async fn create_tag(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Json(request): Json<CreateTagRequest>,
) -> Result<Json<TagResponse>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_service = BlogService::new();
    let tag = blog_service
        .create_tag(&pool, &domain, request)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tag))
}

pub async fn get_tags(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
) -> Result<Json<Vec<TagResponse>>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_service = BlogService::new();
    let tags = blog_service
        .get_tags(&pool, &domain)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tags))
}

// Tracking handlers
pub async fn track_blog_view(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(blog_id): Path<String>,
    Json(request): Json<TrackBlogViewRequest>,
) -> Result<Json<()>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_id = Uuid::parse_str(&blog_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid blog ID".to_string(),
        )
    })?;

    let blog_service = BlogService::new();
    blog_service
        .track_blog_view(
            &pool,
            &domain,
            blog_id,
            request.user_id,
            request.ip_address,
            request.user_agent,
        )
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(()))
}

pub async fn get_blog_stats(
    State(pool): State<DatabasePool>,
    headers: HeaderMap,
    Path(blog_id): Path<String>,
) -> Result<Json<BlogStats>, (axum::http::StatusCode, String)> {
    let domain = extract_domain_from_headers(&headers)
        .map_err(|e| (axum::http::StatusCode::BAD_REQUEST, e.to_string()))?;

    let blog_id = Uuid::parse_str(&blog_id).map_err(|_| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            "Invalid blog ID".to_string(),
        )
    })?;

    let blog_service = BlogService::new();
    let stats = blog_service
        .get_blog_stats(&pool, &domain, blog_id)
        .await
        .map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(stats))
}

// Helper function to extract domain from headers
fn extract_domain_from_headers(headers: &HeaderMap) -> anyhow::Result<String> {
    // In production, you'd extract from Host header or custom header
    // For now, return a default domain
    Ok("example.com".to_string())
}
