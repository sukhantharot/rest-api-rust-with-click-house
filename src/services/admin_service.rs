use crate::database::{get_base_client, DatabasePool};
use crate::handlers::admin_handlers::*;
use crate::migrations;
use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Duration, Utc};
use clickhouse::Client;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct BaseUserClaims {
    sub: String, // user id
    username: String,
    role: Option<String>,
    exp: i64, // expiration time
}

pub struct AdminService {
    jwt_secret: String,
    jwt_expiration_hours: i64,
}

impl AdminService {
    pub fn new() -> Self {
        Self {
            jwt_secret: std::env::var("JWT_SECRET").unwrap_or_else(|_| {
                "your-super-secret-jwt-key-here-change-in-production".to_string()
            }),
            jwt_expiration_hours: std::env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
        }
    }

    // Authenticate base user (system admin)
    pub async fn authenticate_base_user(
        &self,
        pool: &DatabasePool,
        request: BaseUserLoginRequest,
    ) -> Result<BaseUserLoginResponse> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        // Find user by username or email
        let user_rows = base_client
            .query("SELECT id, username, email, password_hash FROM users WHERE (username = ? OR email = ?) AND is_active = true")
            .bind(&request.username_or_email)
            .bind(&request.username_or_email)
            .fetch_all::<(String, String, String, String)>()
            .await?;

        if user_rows.is_empty() {
            return Err(anyhow::anyhow!("Invalid credentials"));
        }

        let user_row = &user_rows[0];
        let user_id = Uuid::parse_str(&user_row.0)?;
        let username = &user_row.1;
        let email = &user_row.2;
        let password_hash = &user_row.3;
        let role = Some("admin".to_string()); // Default role
        let created_at = Utc::now(); // Default time

        // Verify password
        if !verify(&request.password, password_hash)? {
            return Err(anyhow::anyhow!("Invalid credentials"));
        }

        // Generate JWT token
        let expires_at = Utc::now() + Duration::hours(self.jwt_expiration_hours);
        let claims = BaseUserClaims {
            sub: user_id.to_string(),
            username: username.clone(),
            role: role.clone(),
            exp: expires_at.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        // Update last login time
        base_client
            .query("UPDATE users SET last_login_at = now() WHERE id = ?")
            .bind(user_id.to_string())
            .execute()
            .await?;

        Ok(BaseUserLoginResponse {
            token,
            expires_at,
            user: BaseUserResponse {
                id: user_id,
                username: username.clone(),
                email: email.clone(),
                role,
                is_active: true,
                created_at,
            },
        })
    }

    // Get all client connections
    pub async fn get_client_connections(
        &self,
        pool: &DatabasePool,
        limit: Option<u32>,
        offset: Option<u32>,
        is_active: Option<bool>,
    ) -> Result<Vec<ClientConnectResponse>> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        let mut query = "SELECT id, domain, database_url, database_name, is_active, created_at FROM client_connect".to_string();
        let mut params: Vec<String> = Vec::new();

        // Add WHERE clause if filtering by is_active
        if let Some(active) = is_active {
            query.push_str(" WHERE is_active = ?");
            params.push(active.to_string());
        }

        // Add ordering
        query.push_str(" ORDER BY created_at DESC");

        // Add pagination
        if let Some(limit_val) = limit {
            query.push_str(" LIMIT ?");
            params.push(limit_val.to_string());
        }

        if let Some(offset_val) = offset {
            query.push_str(" OFFSET ?");
            params.push(offset_val.to_string());
        }

        let mut clickhouse_query = base_client.query(&query);
        for param in params {
            clickhouse_query = clickhouse_query.bind(param);
        }

        let rows = clickhouse_query
            .fetch_all::<(u64, String, String, String, bool)>()
            .await?;

        let mut connections = Vec::new();
        for row in rows {
            connections.push(ClientConnectResponse {
                id: row.0,
                domain: row.1,
                database_url: row.2,
                database_name: row.3,
                is_active: row.4,
                created_at: Utc::now(), // Default time
                updated_at: Utc::now(), // Default time
            });
        }

        Ok(connections)
    }

    // Create new client connection
    pub async fn create_client_connection(
        &self,
        pool: &DatabasePool,
        request: CreateClientConnectRequest,
    ) -> Result<ClientConnectResponse> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        // Check if domain already exists
        let existing = base_client
            .query("SELECT COUNT(*) FROM client_connect WHERE domain = ?")
            .bind(&request.domain)
            .fetch_one::<u64>()
            .await?;

        if existing > 0 {
            return Err(anyhow::anyhow!("Domain already exists: {}", request.domain));
        }

        // Test connection to the new database
        let test_client = Client::default()
            .with_url(&request.database_url)
            .with_database(&request.database_name);

        // Test query to verify connection
        test_client
            .query("SELECT 1")
            .fetch_all::<u8>()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to connect to database: {}", e))?;

        // Generate ID (for simplicity, using current timestamp)
        let id = Utc::now().timestamp_millis() as u64;
        let now = Utc::now();

        // Insert new client connection
        base_client
            .query("INSERT INTO client_connect (id, domain, database_url, database_name, is_active, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)")
            .bind(id)
            .bind(&request.domain)
            .bind(&request.database_url)
            .bind(&request.database_name)
            .bind(true)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        // Add to connection pool
        crate::database::add_client_connection(
            pool,
            &request.domain,
            &request.database_url,
            &request.database_name,
        )
        .await
        .map_err(|e| anyhow::anyhow!("Failed to add client connection: {}", e))?;

        Ok(ClientConnectResponse {
            id,
            domain: request.domain,
            database_url: request.database_url,
            database_name: request.database_name,
            is_active: true,
            created_at: now,
            updated_at: now,
        })
    }

    // Get specific client connection
    pub async fn get_client_connection(
        &self,
        pool: &DatabasePool,
        id: u64,
    ) -> Result<Option<ClientConnectResponse>> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        let rows = base_client
            .query("SELECT id, domain, database_url, database_name, is_active FROM client_connect WHERE id = ?")
            .bind(id)
            .fetch_all::<(u64, String, String, String, bool)>()
            .await?;

        if rows.is_empty() {
            return Ok(None);
        }

        let row = &rows[0];
        Ok(Some(ClientConnectResponse {
            id: row.0,
            domain: row.1.clone(),
            database_url: row.2.clone(),
            database_name: row.3.clone(),
            is_active: row.4,
            created_at: Utc::now(), // Default time
            updated_at: Utc::now(), // Default time
        }))
    }

    // Update client connection
    pub async fn update_client_connection(
        &self,
        pool: &DatabasePool,
        id: u64,
        request: UpdateClientConnectRequest,
    ) -> Result<ClientConnectResponse> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        // Check if connection exists
        let existing = self.get_client_connection(pool, id).await?;
        let mut existing_conn =
            existing.ok_or_else(|| anyhow::anyhow!("Client connection not found"))?;

        let mut query = "UPDATE client_connect SET updated_at = now()".to_string();
        let mut params: Vec<String> = Vec::new();

        if let Some(domain) = &request.domain {
            query.push_str(", domain = ?");
            params.push(domain.clone());
            existing_conn.domain = domain.clone();
        }

        if let Some(database_url) = &request.database_url {
            query.push_str(", database_url = ?");
            params.push(database_url.clone());
            existing_conn.database_url = database_url.clone();
        }

        if let Some(database_name) = &request.database_name {
            query.push_str(", database_name = ?");
            params.push(database_name.clone());
            existing_conn.database_name = database_name.clone();
        }

        if let Some(is_active) = request.is_active {
            query.push_str(", is_active = ?");
            params.push(is_active.to_string());
            existing_conn.is_active = is_active;
        }

        query.push_str(" WHERE id = ?");
        params.push(id.to_string());

        let mut clickhouse_query = base_client.query(&query);
        for param in params {
            clickhouse_query = clickhouse_query.bind(param);
        }
        clickhouse_query.execute().await?;

        // Update connection pool if needed
        if request.domain.is_some()
            || request.database_url.is_some()
            || request.database_name.is_some()
        {
            // Remove old connection
            crate::database::remove_client_connection(pool, &existing_conn.domain)
                .await
                .map_err(|e| anyhow::anyhow!("Failed to remove client connection: {}", e))?;

            // Add new connection if active
            if existing_conn.is_active {
                crate::database::add_client_connection(
                    pool,
                    &existing_conn.domain,
                    &existing_conn.database_url,
                    &existing_conn.database_name,
                )
                .await
                .map_err(|e| anyhow::anyhow!("Failed to add client connection: {}", e))?;
            }
        }

        existing_conn.updated_at = Utc::now();
        Ok(existing_conn)
    }

    // Delete (deactivate) client connection
    pub async fn delete_client_connection(&self, pool: &DatabasePool, id: u64) -> Result<()> {
        let base_client = get_base_client(pool)
            .await
            .ok_or_else(|| anyhow::anyhow!("Base database client not available"))?;

        // Get connection details first
        let connection = self
            .get_client_connection(pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Client connection not found"))?;

        // Update to inactive
        base_client
            .query("UPDATE client_connect SET is_active = false, updated_at = now() WHERE id = ?")
            .bind(id)
            .execute()
            .await?;

        // Remove from connection pool
        crate::database::remove_client_connection(pool, &connection.domain)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to remove client connection: {}", e))?;

        Ok(())
    }

    // Test client database connection
    pub async fn test_client_connection(
        &self,
        pool: &DatabasePool,
        id: u64,
    ) -> Result<serde_json::Value> {
        let connection = self
            .get_client_connection(pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Client connection not found"))?;

        let test_client = Client::default()
            .with_url(&connection.database_url)
            .with_database(&connection.database_name);

        let start_time = std::time::Instant::now();
        let result = test_client
            .query("SELECT 1 as test")
            .fetch_one::<u8>()
            .await;
        let duration = start_time.elapsed();

        match result {
            Ok(_) => Ok(serde_json::json!({
                "status": "success",
                "domain": connection.domain,
                "database_name": connection.database_name,
                "response_time_ms": duration.as_millis(),
                "message": "Connection successful"
            })),
            Err(e) => Ok(serde_json::json!({
                "status": "error",
                "domain": connection.domain,
                "database_name": connection.database_name,
                "response_time_ms": duration.as_millis(),
                "error": e.to_string()
            })),
        }
    }

    // Run client database migrations
    pub async fn migrate_client_database(&self, pool: &DatabasePool, id: u64) -> Result<()> {
        let connection = self
            .get_client_connection(pool, id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Client connection not found"))?;

        let client = Client::default()
            .with_url(&connection.database_url)
            .with_database(&connection.database_name);

        // Run client migrations
        migrations::run_client_migrations(&client)
            .await
            .map_err(|e| anyhow::anyhow!("Migration failed: {}", e))?;

        tracing::info!(
            "Client database migration completed for domain: {}",
            connection.domain
        );
        Ok(())
    }
}
