use crate::database::DatabasePool;
use crate::models::user::*;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use clickhouse::Client;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

pub struct UserService {
    jwt_secret: String,
    jwt_expiration_hours: i64,
}

impl UserService {
    pub fn new(jwt_secret: String, jwt_expiration_hours: i64) -> Self {
        Self {
            jwt_secret,
            jwt_expiration_hours,
        }
    }

    pub async fn create_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Check if username or email already exists
        let existing_count = client
            .query("SELECT count() FROM users WHERE username = ? OR email = ?")
            .bind(&request.username)
            .bind(&request.email)
            .fetch_one::<u64>()
            .await?;

        if existing_count > 0 {
            return Err("Username or email already exists".into());
        }

        // Hash password
        let password_hash = hash(request.password, DEFAULT_COST)?;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert user
        client
            .query("INSERT INTO users (id, username, email, password_hash, first_name, last_name, is_active, is_verified, role_id, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
            .bind(user_id)
            .bind(&request.username)
            .bind(&request.email)
            .bind(&password_hash)
            .bind(&request.first_name)
            .bind(&request.last_name)
            .bind(true)
            .bind(false)
            .bind(request.role_id)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        // Get the created user with role
        let user = self.get_user_by_id(pool, domain, user_id).await?;

        Ok(user)
    }

    pub async fn get_user_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Get user data - split into smaller queries to avoid tuple size limits
        // First get the basic user info as separate queries
        let user_id_str = client
            .query("SELECT id FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?
            .ok_or("User not found")?;

        let username = client
            .query("SELECT username FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let email = client
            .query("SELECT email FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let first_name = client
            .query("SELECT first_name FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let last_name = client
            .query("SELECT last_name FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let is_active = client
            .query("SELECT is_active FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let is_verified = client
            .query("SELECT is_verified FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let role_id_str = client
            .query("SELECT role_id FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let last_login_at_str = client
            .query("SELECT last_login_at FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        // Parse the values
        let parsed_user_id = Uuid::parse_str(&user_id_str)?;
        let role_id = role_id_str.as_ref().and_then(|s| Uuid::parse_str(s).ok());
        let last_login_at = last_login_at_str
            .as_ref()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        // Get role data if user has a role
        let role = if let Some(rid) = role_id {
            self.get_role_by_id(&client, domain, rid).await.ok()
        } else {
            None
        };

        Ok(UserResponse {
            id: parsed_user_id,
            username,
            email,
            first_name,
            last_name,
            is_active,
            is_verified,
            role,
            last_login_at,
            created_at,
            updated_at,
        })
    }

    pub async fn get_users(
        &self,
        pool: &DatabasePool,
        domain: &str,
        limit: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<UserResponse>, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        // Get user IDs first - use count to avoid single-element tuple
        let user_count = client
            .query("SELECT count() FROM users WHERE domain = ?")
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        let mut user_responses = Vec::new();

        // If we have users, get them one by one to avoid large tuples
        if user_count > 0 {
            for i in 0..limit {
                let idx = i as u64 + offset as u64;
                if idx >= user_count {
                    break;
                }

                // Get user ID at this position as string
                let user_id_str = client
                    .query("SELECT id FROM users WHERE domain = ? ORDER BY created_at DESC LIMIT 1 OFFSET ?")
                    .bind(domain)
                    .bind(idx)
                    .fetch_one::<String>()
                    .await?;

                let user_id = Uuid::parse_str(&user_id_str)?;

                if let Ok(user) = self.get_user_by_id(pool, domain, user_id).await {
                    user_responses.push(user);
                }
            }
        }

        Ok(user_responses)
    }

    pub async fn update_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
        request: UpdateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        let now = Utc::now();

        // Build dynamic update query
        let mut query = "UPDATE users SET updated_at = ?".to_string();
        let mut binds: Vec<String> = vec![now.to_string()];

        if let Some(username) = &request.username {
            query.push_str(", username = ?");
            binds.push(username.clone());
        }
        if let Some(email) = &request.email {
            query.push_str(", email = ?");
            binds.push(email.clone());
        }
        if let Some(ref first_name) = request.first_name {
            query.push_str(", first_name = ?");
            binds.push(first_name.clone());
        }
        if let Some(ref last_name) = request.last_name {
            query.push_str(", last_name = ?");
            binds.push(last_name.clone());
        }
        if let Some(is_active) = request.is_active {
            query.push_str(", is_active = ?");
            binds.push(is_active.to_string());
        }
        if let Some(role_id) = request.role_id {
            query.push_str(", role_id = ?");
            binds.push(role_id.to_string());
        }

        query.push_str(" WHERE id = ?");
        binds.push(user_id.to_string());

        let mut clickhouse_query = client.query(&query).bind(now);
        for bind in binds {
            clickhouse_query = clickhouse_query.bind(bind);
        }
        clickhouse_query.execute().await?;

        // Get updated user
        self.get_user_by_id(pool, domain, user_id).await
    }

    pub async fn delete_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        client
            .query("DELETE FROM users WHERE id = ?")
            .bind(user_id)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn authenticate_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: LoginRequest,
    ) -> Result<LoginResponse, Box<dyn std::error::Error>> {
        let client = self.get_client_by_domain(pool, domain).await?;

        // Find user by username or email - check if exists first
        let user_exists = client
            .query("SELECT count() FROM users WHERE (username = ? OR email = ?) AND is_active = ? AND domain = ?")
            .bind(&request.username_or_email)
            .bind(&request.username_or_email)
            .bind(true)
            .bind(domain)
            .fetch_one::<u64>()
            .await?;

        if user_exists == 0 {
            return Err("Invalid credentials".into());
        }

        // Get user ID as string
        let user_id_str = client
            .query("SELECT id FROM users WHERE (username = ? OR email = ?) AND is_active = ? AND domain = ?")
            .bind(&request.username_or_email)
            .bind(&request.username_or_email)
            .bind(true)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let user_id = Uuid::parse_str(&user_id_str)?;

        let user = self.get_user_by_id(pool, domain, user_id).await?;

        // Get password hash from database
        let password_hash = client
            .query("SELECT password_hash FROM users WHERE id = ? AND domain = ?")
            .bind(user_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        // Verify password
        if !verify(&request.password, &password_hash)? {
            return Err("Invalid credentials".into());
        }

        // Update last login
        let now = Utc::now();
        client
            .query("UPDATE users SET last_login_at = ?, updated_at = ? WHERE id = ? AND domain = ?")
            .bind(now)
            .bind(now)
            .bind(user_id)
            .bind(domain)
            .execute()
            .await?;

        // Generate JWT token
        let expiration = now + chrono::Duration::hours(self.jwt_expiration_hours);
        let claims = Claims {
            sub: user_id.to_string(),
            exp: expiration.timestamp() as usize,
            iat: now.timestamp() as usize,
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )?;

        Ok(LoginResponse {
            user,
            token,
            token_type: "Bearer".to_string(),
            expires_at: expiration,
        })
    }

    async fn get_client_by_domain(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> Result<Client, Box<dyn std::error::Error>> {
        use crate::database::get_client_by_domain;
        get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| format!("No database connection found for domain: {}", domain).into())
    }

    async fn get_role_by_id(
        &self,
        client: &Client,
        domain: &str,
        role_id: Uuid,
    ) -> Result<Role, Box<dyn std::error::Error>> {
        // Get role data as separate queries to avoid tuple size limits
        let role_id_str = client
            .query("SELECT id FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let name = client
            .query("SELECT name FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let description = client
            .query("SELECT description FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_optional::<String>()
            .await?;

        let is_active = client
            .query("SELECT is_active FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<bool>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM roles WHERE id = ? AND domain = ?")
            .bind(role_id)
            .bind(domain)
            .fetch_one::<String>()
            .await?;

        let parsed_role_id = Uuid::parse_str(&role_id_str)?;
        let created_at = DateTime::parse_from_rfc3339(&created_at_str)?.with_timezone(&Utc);
        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)?.with_timezone(&Utc);

        Ok(Role {
            id: parsed_role_id,
            name,
            description,
            is_active,
            created_at,
            updated_at,
        })
    }
}
