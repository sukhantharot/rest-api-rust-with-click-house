use crate::config::Config;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Pool, Postgres, Row};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

pub type DatabasePool = Arc<RwLock<HashMap<String, PgPool>>>;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ClientConnect {
    pub id: Uuid,
    pub domain: String,
    pub database_url: String,
    pub database_name: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct BaseUser {
    pub id: Uuid,
    pub username: String,
    pub email: String,
    pub password_hash: String,
    pub is_active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

pub async fn init_database(config: &Config) -> Result<DatabasePool, Box<dyn std::error::Error>> {
    let mut pool = HashMap::new();

    tracing::info!(
        "Connecting to PostgreSQL at {}:{}",
        config.database.host,
        config.database.port
    );

    // Initialize base database connection pool
    let base_pool = PgPool::connect(&config.database.url).await?;

    // Test the connection
    tracing::info!("Testing database connection...");
    match sqlx::query("SELECT 1").fetch_one(&base_pool).await {
        Ok(_) => tracing::info!("✅ Database connection successful"),
        Err(e) => {
            tracing::error!("❌ Database connection failed: {}", e);
            return Err(Box::new(e));
        }
    }

    pool.insert("base".to_string(), base_pool);

    let pool = Arc::new(RwLock::new(pool));

    // Load client connections from base database
    load_client_connections(&pool).await?;

    Ok(pool)
}

pub async fn load_client_connections(
    pool: &DatabasePool,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_pool = {
        let pool_read = pool.read().await;
        pool_read.get("base").unwrap().clone()
    };

    let client_connections: Vec<ClientConnect> = sqlx::query_as(
        "SELECT id, domain, database_url, database_name, is_active, created_at, updated_at 
         FROM client_connect WHERE is_active = $1",
    )
    .bind(true)
    .fetch_all(&base_pool)
    .await?;

    let mut pool_write = pool.write().await;
    for client_conn in client_connections {
        match PgPool::connect(&client_conn.database_url).await {
            Ok(client_pool) => {
                // Test the connection
                if let Ok(_) = sqlx::query("SELECT 1").fetch_one(&client_pool).await {
                    pool_write.insert(client_conn.domain.clone(), client_pool);
                    tracing::info!(
                        "✅ Loaded client connection for domain: {}",
                        client_conn.domain
                    );
                } else {
                    tracing::warn!(
                        "⚠️  Failed to test client database connection: {}",
                        client_conn.domain
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    "⚠️  Failed to connect to client database {}: {}",
                    client_conn.domain,
                    e
                );
            }
        }
    }

    Ok(())
}

pub async fn get_client_by_domain(pool: &DatabasePool, domain: &str) -> Option<PgPool> {
    let pool_read = pool.read().await;
    pool_read.get(domain).cloned()
}

pub async fn get_base_pool(pool: &DatabasePool) -> Option<PgPool> {
    let pool_read = pool.read().await;
    pool_read.get("base").cloned()
}

// Alias for compatibility
pub async fn get_base_client(pool: &DatabasePool) -> Option<PgPool> {
    get_base_pool(pool).await
}

pub async fn add_client_connection(
    pool: &DatabasePool,
    domain: &str,
    database_url: &str,
    database_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_pool = get_base_pool(pool)
        .await
        .ok_or("Base database pool not available")?;

    // Insert into base database
    let client_id = Uuid::new_v4();
    let now = chrono::Utc::now();

    sqlx::query(
        "INSERT INTO client_connect (id, domain, database_url, database_name, is_active, created_at, updated_at) 
         VALUES ($1, $2, $3, $4, $5, $6, $7)"
    )
    .bind(client_id)
    .bind(domain)
    .bind(database_url)
    .bind(database_name)
    .bind(true)
    .bind(now)
    .bind(now)
    .execute(&base_pool)
    .await?;

    // Add to connection pool
    let client_pool = PgPool::connect(database_url).await?;

    let mut pool_write = pool.write().await;
    pool_write.insert(domain.to_string(), client_pool);

    tracing::info!("✅ Added client connection for domain: {}", domain);

    Ok(())
}

pub async fn remove_client_connection(
    pool: &DatabasePool,
    domain: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let base_pool = get_base_pool(pool)
        .await
        .ok_or("Base database pool not available")?;

    // Mark as inactive in base database
    let now = chrono::Utc::now();
    sqlx::query("UPDATE client_connect SET is_active = $1, updated_at = $2 WHERE domain = $3")
        .bind(false)
        .bind(now)
        .bind(domain)
        .execute(&base_pool)
        .await?;

    // Remove from connection pool
    let mut pool_write = pool.write().await;
    if let Some(removed_pool) = pool_write.remove(domain) {
        removed_pool.close().await;
        tracing::info!("✅ Removed client connection for domain: {}", domain);
    }

    Ok(())
}

// Helper function to get connection pool for a domain
pub async fn get_pool_for_domain(
    pool: &DatabasePool,
    domain: &str,
) -> Result<PgPool, anyhow::Error> {
    get_client_by_domain(pool, domain)
        .await
        .ok_or_else(|| anyhow::anyhow!("No database pool found for domain: {}", domain))
}
