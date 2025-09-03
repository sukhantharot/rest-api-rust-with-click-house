use crate::config::Config;
use clickhouse::{Client, Row};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use url::Url;

pub type DatabasePool = Arc<RwLock<HashMap<String, Client>>>;

// Helper function to convert HTTPS URLs to HTTP for ClickHouse client compatibility
fn convert_clickhouse_url(url_str: &str) -> Result<String, Box<dyn std::error::Error>> {
    let url = Url::parse(url_str)?;

    if url.scheme() == "https" {
        // Convert HTTPS to HTTP (ClickHouse client doesn't support HTTPS URLs directly)
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

        tracing::warn!("⚠️  Converted HTTPS URL to HTTP for ClickHouse client compatibility");
        tracing::debug!("   Original: {}", url_str);
        tracing::debug!("   Converted: {}", http_url);

        Ok(http_url)
    } else {
        // Already HTTP, return as-is
        Ok(url_str.to_string())
    }
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct ClientConnect {
    pub id: u64,
    pub domain: String,
    pub database_url: String,
    pub database_name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct BaseUser {
    pub id: u64,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn init_database(config: &Config) -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let mut pool = HashMap::new();

    // Initialize base database connection using HTTPS URL
    // Clean host (remove https:// prefix if exists)
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

    tracing::info!(
        "Connecting to ClickHouse with URL: https://{}:***@{}/{}",
        config.clickhouse.base_user,
        clean_host,
        config.clickhouse.base_db
    );

    let base_client = Client::default().with_url(https_url);

    // Test the connection (disable for now to allow server to start)
    tracing::info!("Testing database connection...");
    match base_client.query("SELECT 1").fetch_all::<u8>().await {
        Ok(_) => tracing::info!("✅ Database connection successful"),
        Err(e) => {
            tracing::warn!("⚠️  Database connection failed: {}", e);
            tracing::info!("💡 Server will continue running without database for development");
        }
    }

    pool.insert("base".to_string(), base_client);

    let pool = Arc::new(RwLock::new(pool));

    // Load client connections from base database
    load_client_connections(&pool).await?;

    Ok(pool)
}

pub async fn load_client_connections(
    pool: &DatabasePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_client = {
        let pool_read = pool.read().await;
        pool_read.get("base").unwrap().clone()
    };

    let client_connections = base_client
        .query("SELECT id, domain, database_url, database_name, is_active, created_at, updated_at FROM client_connect WHERE is_active = ?")
        .bind(true)
        .fetch_all::<ClientConnect>()
        .await?;

    let mut pool_write = pool.write().await;
    for client_conn in client_connections {
        let client = Client::default()
            .with_url(&client_conn.database_url)
            .with_database(&client_conn.database_name);

        // Test the connection
        if let Ok(_) = client.query("SELECT 1").fetch_all::<u8>().await {
            pool_write.insert(client_conn.domain.clone(), client);
            tracing::info!(
                "Loaded client connection for domain: {}",
                client_conn.domain
            );
        } else {
            tracing::warn!(
                "Failed to connect to client database: {}",
                client_conn.domain
            );
        }
    }

    Ok(())
}

pub async fn get_client_by_domain(pool: &DatabasePool, domain: &str) -> Option<Client> {
    let pool_read = pool.read().await;
    pool_read.get(domain).cloned()
}

pub async fn get_base_client(pool: &DatabasePool) -> Option<Client> {
    let pool_read = pool.read().await;
    pool_read.get("base").cloned()
}

pub async fn add_client_connection(
    pool: &DatabasePool,
    domain: &str,
    database_url: &str,
    database_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_client = get_base_client(pool)
        .await
        .ok_or("Base database client not available")?;

    // Insert into base database
    base_client
        .query("INSERT INTO client_connect (domain, database_url, database_name, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, now(), now())")
        .bind(domain)
        .bind(database_url)
        .bind(database_name)
        .bind(true)
        .execute()
        .await?;

    // Add to connection pool
    let client = Client::default()
        .with_url(database_url)
        .with_database(database_name);

    let mut pool_write = pool.write().await;
    pool_write.insert(domain.to_string(), client);

    Ok(())
}

pub async fn remove_client_connection(
    pool: &DatabasePool,
    domain: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_client = get_base_client(pool)
        .await
        .ok_or("Base database client not available")?;

    // Mark as inactive in base database
    base_client
        .query("UPDATE client_connect SET is_active = ?, updated_at = now() WHERE domain = ?")
        .bind(false)
        .bind(domain)
        .execute()
        .await?;

    // Remove from connection pool
    let mut pool_write = pool.write().await;
    pool_write.remove(domain);

    Ok(())
}
