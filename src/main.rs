mod config;
mod database;
mod handlers;
mod middleware;
mod migrations;
mod models;
mod services;

use axum::{Router, routing::get};
use std::collections::HashMap;
use std::net::SocketAddr;
use tower::{Layer, ServiceBuilder};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing_subscriber::{
    EnvFilter, Layer as _,
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

#[tokio::main]
async fn main() {
    // Load configuration first to get logging settings
    let config = config::load_config()
        .await
        .expect("Failed to load configuration");

    // Initialize enhanced tracing based on configuration
    let env_filter = config.logging.create_env_filter();

    let fmt_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_target(false)
        .with_thread_ids(false)
        .with_file(false)
        .with_line_number(false)
        .pretty();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .init();

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        log_level = %config.logging.level,
        json_format = %config.logging.json_format,
        request_logging = %config.logging.request_logging,
        slow_threshold_ms = %config.logging.slow_request_threshold_ms,
        "🚀 Starting REST API Server with enhanced logging"
    );

    // Initialize database connections (continue even if failed)
    let db_pool = match database::init_database(&config).await {
        Ok(pool) => {
            tracing::info!("✅ Database pool initialized successfully");
            pool
        }
        Err(e) => {
            tracing::warn!("⚠️  Database initialization failed: {}", e);
            tracing::info!("💡 Server will continue running in development mode");
            // Return empty pool for development
            std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::<
                String,
                clickhouse::Client,
            >::new()))
        }
    };

    // Run migrations (skip if database unavailable)
    match run_migrations(&db_pool, &config).await {
        Ok(()) => tracing::info!("✅ Migrations completed successfully"),
        Err(e) => {
            tracing::warn!("⚠️  Migration failed: {}", e);
            tracing::info!("💡 Continuing without migrations for development");
        }
    }

    // Initialize task scheduler
    let task_service = services::TaskService::new();
    services::register_builtin_handlers(&task_service).await;

    let task_scheduler = services::TaskScheduler::new(task_service, db_pool.clone());

    // Start task workers for available domains
    // In a production system, you'd start workers for all active domains
    // TODO: Start workers based on actual configured domains
    // if let Err(e) = task_scheduler.start_worker_for_domain("your-domain.com").await {
    //     tracing::warn!("Failed to start task worker: {}", e);
    // }

    // Create the main router with comprehensive middleware stack
    let mut app = Router::new()
        .route("/", get(root))
        .route("/health", get(health_check))
        // Admin routes (without domain middleware - for system administration)
        .merge(handlers::admin_handlers::admin_routes())
        // Client routes (with domain middleware)
        .merge(
            Router::new()
                .merge(handlers::user_handlers::user_routes())
                .merge(handlers::role_handlers::role_routes())
                .merge(handlers::permission_handlers::permission_routes())
                .merge(handlers::blog_handlers::blog_routes())
                .merge(handlers::task_handlers::task_routes())
                .layer(axum::middleware::from_fn_with_state(
                    db_pool.clone(),
                    middleware::domain_middleware::domain_middleware,
                )),
        )
        // Add CORS layer
        .layer(CorsLayer::permissive())
        // Add Tower HTTP tracing layer
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(|request: &axum::extract::Request| {
                    tracing::info_span!(
                        "http_request",
                        method = %request.method(),
                        uri = %request.uri(),
                        version = ?request.version(),
                    )
                })
                .on_request(|_request: &axum::extract::Request, _span: &tracing::Span| {
                    tracing::debug!("Started processing request");
                })
                .on_response(
                    |response: &axum::response::Response,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::debug!(
                            status = %response.status(),
                            latency_ms = %latency.as_millis(),
                            "Finished processing request"
                        );
                    },
                )
                .on_failure(
                    |error: tower_http::classify::ServerErrorsFailureClass,
                     latency: std::time::Duration,
                     _span: &tracing::Span| {
                        tracing::error!(
                            error = %error,
                            latency_ms = %latency.as_millis(),
                            "Request failed"
                        );
                    },
                ),
        )
        .with_state(db_pool.clone());

    // Conditionally add request logging middleware if enabled
    if config.logging.request_logging {
        app = app.layer(axum::middleware::from_fn(
            middleware::request_logging::simple_request_logging_middleware,
        ));
    }

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], config.server.port));

    tracing::info!(
        address = %addr,
        port = config.server.port,
        "🌐 Server starting..."
    );

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!(
        address = %addr,
        "✅ Server successfully bound to address"
    );

    tracing::info!("🎯 API Endpoints available:",);
    tracing::info!("   📊 Health Check: GET  http://{}/health", addr);
    tracing::info!("   🏠 Root:         GET  http://{}/", addr);
    tracing::info!("   👑 Admin:        POST http://{}/admin/auth/login", addr);
    tracing::info!("   👤 Users:        GET  http://{}/users", addr);
    tracing::info!("   📝 Blog:         GET  http://{}/blogs", addr);

    tracing::info!("🚀 Server is ready to accept connections!");

    if let Err(e) = axum::serve(listener, app).await {
        tracing::error!(error = %e, "❌ Server failed to start");
        std::process::exit(1);
    }
}

async fn run_migrations(
    db_pool: &database::DatabasePool,
    config: &config::Config,
) -> Result<(), Box<dyn std::error::Error>> {
    // Run base database migrations (skip if no base client available)
    let base_client = match database::get_base_client(db_pool).await {
        Some(client) => client,
        None => {
            tracing::warn!("No base database client available, skipping migrations");
            return Ok(());
        }
    };
    migrations::run_base_migrations(&base_client).await?;

    // For now, we'll also run client migrations on the base database for testing
    // In production, you would run this on each client database
    migrations::run_client_migrations(&base_client).await?;

    Ok(())
}

async fn root() -> &'static str {
    "REST API with Rust and ClickHouse - Multi-tenant System"
}

async fn health_check() -> &'static str {
    "OK"
}
