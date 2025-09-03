use axum::{
    extract::{ConnectInfo, MatchedPath, Request},
    middleware::Next,
    response::Response,
};
use std::{net::SocketAddr, time::Instant};
use tracing::{info, warn, Instrument};
use uuid::Uuid;

/// Middleware for logging HTTP requests with timing information
pub async fn request_logging_middleware(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    matched_path: Option<MatchedPath>,
    mut request: Request,
    next: Next,
) -> Response {
    // Generate unique request ID
    let request_id = Uuid::new_v4().to_string();

    // Get request details
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = matched_path
        .as_ref()
        .map(|path| path.as_str())
        .unwrap_or(uri.path());
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("-")
        .to_string();
    let content_length = request
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("0")
        .to_string();

    // Add request ID to headers for downstream services
    request
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());

    // Record start time
    let start_time = Instant::now();

    // Create span for request tracing
    let span = tracing::info_span!(
        "http_request",
        request_id = %request_id,
        method = %method,
        path = %path,
        client_ip = %addr.ip(),
        user_agent = %user_agent,
    );

    // Log incoming request
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        path = %path,
        client_ip = %addr.ip(),
        user_agent = %user_agent,
        content_length = %content_length,
        "📥 Incoming request"
    );

    // Execute the request
    let response = async move { next.run(request).await }
        .instrument(span)
        .await;

    // Calculate duration
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis();

    // Get response details
    let status_code = response.status();
    let response_content_length = response
        .headers()
        .get("content-length")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("0");

    // Determine log level based on status code
    let is_error = status_code.is_client_error() || status_code.is_server_error();
    let is_slow = duration_ms > 1000; // Consider requests over 1s as slow

    // Log response with appropriate level
    if is_error {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            client_ip = %addr.ip(),
            response_size = %response_content_length,
            "❌ Request completed with error"
        );
    } else if is_slow {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            client_ip = %addr.ip(),
            response_size = %response_content_length,
            "🐌 Slow request completed"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            client_ip = %addr.ip(),
            response_size = %response_content_length,
            "✅ Request completed"
        );
    }

    // Add performance metrics to response headers (optional - remove in production if needed)
    let mut response = response;
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());
    response.headers_mut().insert(
        "x-response-time",
        format!("{}ms", duration_ms).parse().unwrap(),
    );

    response
}

/// Middleware for logging requests without ConnectInfo (for cases where it's not available)
pub async fn simple_request_logging_middleware(
    matched_path: Option<MatchedPath>,
    mut request: Request,
    next: Next,
) -> Response {
    // Generate unique request ID
    let request_id = Uuid::new_v4().to_string();

    // Get request details
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = matched_path
        .as_ref()
        .map(|path| path.as_str())
        .unwrap_or(uri.path());
    let user_agent = request
        .headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("-")
        .to_string();

    // Add request ID to headers
    request
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());

    let start_time = Instant::now();

    // Log incoming request
    info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        path = %path,
        user_agent = %user_agent,
        "📥 Incoming request"
    );

    // Execute request
    let response = next.run(request).await;

    // Calculate duration and log response
    let duration = start_time.elapsed();
    let duration_ms = duration.as_millis();
    let status_code = response.status();

    if status_code.is_client_error() || status_code.is_server_error() {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            "❌ Request completed with error"
        );
    } else if duration_ms > 1000 {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            "🐌 Slow request completed"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            path = %path,
            status = %status_code,
            duration_ms = %duration_ms,
            "✅ Request completed"
        );
    }

    let mut response = response;
    response
        .headers_mut()
        .insert("x-request-id", request_id.parse().unwrap());
    response.headers_mut().insert(
        "x-response-time",
        format!("{}ms", duration_ms).parse().unwrap(),
    );

    response
}
