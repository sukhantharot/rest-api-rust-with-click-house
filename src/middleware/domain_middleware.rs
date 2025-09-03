use crate::database::{get_client_by_domain, DatabasePool};
use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::sync::Arc;

pub async fn domain_middleware(
    State(pool): State<DatabasePool>,
    mut req: Request,
    next: Next,
) -> Response {
    // Extract domain from Host header or custom header
    let domain = extract_domain(&req);

    if domain.is_none() {
        return (
            StatusCode::BAD_REQUEST,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error": "Missing domain information"}"#,
        )
            .into_response();
    }

    let domain = domain.unwrap();

    // Check if we have a connection for this domain
    let client = get_client_by_domain(&pool, &domain).await;

    if client.is_none() {
        return (
            StatusCode::NOT_FOUND,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error": "Domain not found"}"#,
        )
            .into_response();
    }

    // Add domain to request extensions for handlers to use
    req.extensions_mut().insert(domain);

    next.run(req).await
}

fn extract_domain(req: &Request) -> Option<String> {
    // Try to get domain from Host header first
    if let Some(host) = req.headers().get(header::HOST) {
        if let Ok(host_str) = host.to_str() {
            // Remove port if present
            return Some(host_str.split(':').next()?.to_string());
        }
    }

    // Try custom tenant domain header
    if let Some(domain) = req.headers().get("x-tenant-domain") {
        if let Ok(domain_str) = domain.to_str() {
            return Some(domain_str.to_string());
        }
    }

    None
}
