use clap::{Arg, Command};
use clickhouse::Client;
use dotenvy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use tracing_subscriber::{fmt, EnvFilter};
use url::Url;

type DatabasePool = Arc<RwLock<HashMap<String, Client>>>;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Config {
    clickhouse: ClickHouseConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClickHouseConfig {
    base_url: String,
    base_db: String,
    base_host: String,
    base_password: String,
    base_user: String,
}

#[derive(Error, Debug)]
pub enum MigrationError {
    #[error("Database connection failed: {0}")]
    DatabaseConnection(String),
    #[error("Configuration error: {0}")]
    Config(String),
    #[error("Migration failed: {0}")]
    Migration(String),
    #[error("Domain not found: {0}")]
    DomainNotFound(String),
    #[error("No active client connections found")]
    NoActiveConnections,
}

fn build_cli() -> Command {
    Command::new("migrate")
        .about("🗃️  Database Migration Tool for ClickHouse Multi-tenant System")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(Command::new("base").about("Run base database migrations"))
        .subcommand(
            Command::new("client")
                .about("Run client migrations")
                .arg(
                    Arg::new("domain")
                        .long("domain")
                        .short('d')
                        .value_name("DOMAIN")
                        .help("Run migrations for specific domain")
                        .conflicts_with("url"),
                )
                .arg(
                    Arg::new("url")
                        .long("url")
                        .short('u')
                        .value_name("DATABASE_URL")
                        .help("Run migrations for specific database URL")
                        .conflicts_with("domain"),
                ),
        )
        .subcommand(Command::new("all").about("Run all migrations (base + all clients)"))
}

// Helper functions from main crate (inlined for binary)
async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config {
        clickhouse: ClickHouseConfig {
            base_url: env::var("CLICKHOUSE_URL").unwrap_or_else(|_| {
                "http://clickhouse-production-71f9.up.railway.app:8123".to_string()
            }),
            base_db: env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "railway".to_string()),
            base_host: env::var("CLICKHOUSE_HOST")
                .unwrap_or_else(|_| "clickhouse-production-71f9.up.railway.app".to_string()),
            base_password: env::var("CLICKHOUSE_PASSWORD")
                .unwrap_or_else(|_| "vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg".to_string()),
            base_user: env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "clickhouse".to_string()),
        },
    };

    Ok(config)
}

fn convert_clickhouse_url(url_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = Url::parse(url_str)?;

    if url.scheme() == "https" {
        let host = url.host_str().ok_or("Invalid host")?;
        let port = url.port().unwrap_or(443);
        let username = url.username();
        let password = url.password().unwrap_or("");
        let database = url.path().trim_start_matches('/');

        // Keep the same port but use HTTP protocol
        let http_port = port;

        let http_url = if username.is_empty() {
            format!("http://{}:{}/{}", host, http_port, database)
        } else {
            format!(
                "http://{}:{}@{}:{}/{}",
                username, password, host, http_port, database
            )
        };

        warn!("⚠️  Converted HTTPS URL to HTTP for ClickHouse client compatibility");

        Ok(http_url)
    } else {
        Ok(url_str.to_string())
    }
}

async fn init_database(config: &Config) -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let mut pool = HashMap::new();

    let clean_host = config
        .clickhouse
        .base_host
        .strip_prefix("https://")
        .unwrap_or(&config.clickhouse.base_host);

    let https_url = format!(
        "https://{}:{}@{}:443/{}",
        config.clickhouse.base_user,
        config.clickhouse.base_password,
        clean_host,
        config.clickhouse.base_db
    );

    let converted_url = convert_clickhouse_url(&https_url)?;
    let base_client = Client::default().with_url(converted_url);

    // Test the connection
    info!("Testing database connection...");
    match base_client.query("SELECT 1").fetch_all::<u8>().await {
        Ok(_) => info!("✅ Database connection successful"),
        Err(e) => {
            warn!("⚠️  Database connection failed: {}", e);
            return Err(e.into());
        }
    }

    pool.insert("base".to_string(), base_client);
    Ok(Arc::new(RwLock::new(pool)))
}

async fn get_base_client(pool: &DatabasePool) -> Option<Client> {
    let pool_read = pool.read().await;
    pool_read.get("base").cloned()
}

// Migration functions (inlined from migrations module)
async fn run_base_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS client_connect (
                id UInt64,
                domain String,
                database_url String,
                database_name String,
                is_active Bool DEFAULT true,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id String,
                username String,
                email String,
                password_hash String,
                first_name Nullable(String),
                last_name Nullable(String),
                is_active Bool DEFAULT true,
                is_verified Bool DEFAULT false,
                role Nullable(String),
                last_login_at Nullable(DateTime),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    info!("Base database migrations completed successfully");
    Ok(())
}

async fn run_client_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Create users table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS users (
                id String,
                username String,
                email String,
                password_hash String,
                first_name Nullable(String),
                last_name Nullable(String),
                is_active Bool DEFAULT true,
                is_verified Bool DEFAULT false,
                role_id Nullable(String),
                last_login_at Nullable(DateTime),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create roles table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS roles (
                id String,
                name String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                domain String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (domain, name)
            "#,
        )
        .execute()
        .await?;

    // Create permissions table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS permissions (
                id String,
                name String,
                resource String,
                action String,
                description Nullable(String),
                is_active Bool DEFAULT true,
                domain String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (domain, resource, action)
            "#,
        )
        .execute()
        .await?;

    // Create tasks table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id String,
                task_type String,
                payload String,
                status String,
                priority UInt8,
                max_attempts UInt32 DEFAULT 3,
                attempts UInt32 DEFAULT 0,
                scheduled_at DateTime,
                started_at Nullable(DateTime),
                completed_at Nullable(DateTime),
                failed_at Nullable(DateTime),
                error_message Nullable(String),
                created_by Nullable(String),
                domain String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY (status, priority, scheduled_at)
            "#,
        )
        .execute()
        .await?;

    info!("Client database migrations completed successfully");
    Ok(())
}

async fn initialize_tracing() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(env_filter)
        .with_target(false)
        .pretty()
        .init();
}

#[tokio::main]
async fn main() -> Result<(), MigrationError> {
    // Initialize tracing first
    initialize_tracing().await;

    // Parse CLI arguments
    let matches = build_cli().get_matches();

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "🚀 Starting ClickHouse Migration Tool"
    );

    // Load configuration
    let config = load_config()
        .await
        .map_err(|e| MigrationError::Config(e.to_string()))?;

    info!("✅ Configuration loaded successfully");

    // Initialize database pool
    let db_pool = init_database(&config)
        .await
        .map_err(|e| MigrationError::DatabaseConnection(e.to_string()))?;

    info!("✅ Database connection established");

    // Execute commands
    match matches.subcommand() {
        Some(("base", _)) => {
            info!("🚀 Running Base Database Migrations...");
            run_base_migration(&db_pool).await?;
            info!("✅ Base migration completed successfully!");
        }
        Some(("client", sub_matches)) => {
            if let Some(domain) = sub_matches.get_one::<String>("domain") {
                info!(
                    "🚀 Running Client Database Migrations for domain: {}",
                    domain
                );
                run_client_migration_by_domain(&db_pool, domain).await?;
            } else if let Some(url) = sub_matches.get_one::<String>("url") {
                info!("🚀 Running Client Database Migrations for URL");
                run_client_migration_by_url(&db_pool, url).await?;
            } else {
                error!("❌ Client migration requires either --domain or --url argument");
                return Err(MigrationError::Config(
                    "Missing required argument".to_string(),
                ));
            }
            info!("✅ Client migration completed successfully!");
        }
        Some(("all", _)) => {
            info!("🚀 Running All Migrations...");

            info!("📊 Running base migration...");
            run_base_migration(&db_pool).await?;

            info!("📊 Running client migrations for all domains...");
            run_all_client_migrations(&db_pool).await?;

            info!("✅ All migrations completed successfully!");
        }
        _ => {
            error!("❌ No command specified. Use --help for usage information.");
            return Err(MigrationError::Config("No command specified".to_string()));
        }
    }

    Ok(())
}

async fn run_base_migration(db_pool: &DatabasePool) -> Result<(), MigrationError> {
    let base_client = get_base_client(db_pool)
        .await
        .ok_or(MigrationError::Config(
            "No base database client available".to_string(),
        ))?;

    info!("🔄 Running base database migrations...");

    run_base_migrations(&base_client)
        .await
        .map_err(|e| MigrationError::Migration(e.to_string()))?;

    info!("✅ Base migrations completed");
    Ok(())
}

async fn run_client_migration_by_url(
    db_pool: &DatabasePool,
    database_url: &str,
) -> Result<(), MigrationError> {
    info!("🔄 Connecting to client database...");

    let client = Client::default().with_url(database_url);

    // Test connection first
    match client.query("SELECT 1").fetch_one::<u8>().await {
        Ok(_) => info!("✅ Connection to client database established"),
        Err(e) => {
            error!("❌ Failed to connect to client database: {}", e);
            return Err(MigrationError::DatabaseConnection(e.to_string()));
        }
    }

    info!("🔄 Running client database migrations...");

    run_client_migrations(&client)
        .await
        .map_err(|e| MigrationError::Migration(e.to_string()))?;

    info!("✅ Client migrations completed");
    Ok(())
}

async fn run_client_migration_by_domain(
    db_pool: &DatabasePool,
    domain: &str,
) -> Result<(), MigrationError> {
    info!("🔍 Looking up database URL for domain: {}", domain);

    let base_client = get_base_client(db_pool)
        .await
        .ok_or(MigrationError::Config(
            "No base database client available".to_string(),
        ))?;

    #[derive(clickhouse::Row, serde::Deserialize)]
    struct ClientConnection {
        database_url: String,
    }

    let query = "SELECT database_url FROM client_connect WHERE domain = ? AND is_active = true";
    let result = base_client
        .query(query)
        .bind(domain)
        .fetch_one::<ClientConnection>()
        .await;

    match result {
        Ok(conn) => {
            info!("📊 Found database URL for domain '{}'", domain);
            run_client_migration_by_url(db_pool, &conn.database_url).await
        }
        Err(_) => {
            error!("❌ Domain '{}' not found in client_connect table", domain);
            Err(MigrationError::DomainNotFound(domain.to_string()))
        }
    }
}

async fn run_all_client_migrations(db_pool: &DatabasePool) -> Result<(), MigrationError> {
    info!("🔍 Loading all active client connections...");

    let base_client = get_base_client(db_pool)
        .await
        .ok_or(MigrationError::Config(
            "No base database client available".to_string(),
        ))?;

    #[derive(clickhouse::Row, serde::Deserialize)]
    struct ClientConnection {
        domain: String,
        database_url: String,
    }

    let query = "SELECT domain, database_url FROM client_connect WHERE is_active = true";
    let rows = base_client
        .query(query)
        .fetch_all::<ClientConnection>()
        .await
        .map_err(|e| MigrationError::DatabaseConnection(e.to_string()))?;

    if rows.is_empty() {
        warn!("ℹ️  No active client connections found");
        return Err(MigrationError::NoActiveConnections);
    }

    info!("📊 Found {} active client connection(s)", rows.len());

    let mut success_count = 0;
    let mut error_count = 0;

    for conn in rows {
        info!("🔄 Migrating domain: {}", conn.domain);

        match run_client_migration_by_url(db_pool, &conn.database_url).await {
            Ok(()) => {
                info!("  ✅ Migration successful for {}", conn.domain);
                success_count += 1;
            }
            Err(e) => {
                error!("  ❌ Migration failed for {}: {}", conn.domain, e);
                error_count += 1;
            }
        }
    }

    info!(
        "🏁 Migration summary: {} successful, {} failed",
        success_count, error_count
    );

    if error_count > 0 {
        warn!("⚠️  Some migrations failed. Check logs above for details.");
    }

    Ok(())
}
