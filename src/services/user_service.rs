use crate::database::DatabasePool;
use crate::models::user::*;
use bcrypt::{hash, verify, DEFAULT_COST};
use chrono::{DateTime, Utc};
use sqlx::Row;
use std::collections::HashMap;
use uuid::Uuid;

pub struct UserService;

impl UserService {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        // Check if user already exists
        let existing_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM users WHERE (username = $1 OR email = $2)")
                .bind(&request.username)
                .bind(&request.email)
                .fetch_one(&pg_pool)
                .await?;

        if existing_count > 0 {
            return Err("User with this username or email already exists".into());
        }

        // Hash password
        let password_hash = hash(request.password, DEFAULT_COST)?;

        let user_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert user
        sqlx::query(
            r#"
            INSERT INTO users (
                id, username, email, password_hash, first_name, last_name, 
                is_active, is_verified, role_id, created_at, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
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
        .execute(&pg_pool)
        .await?;

        // Get the created user
        let user = self.get_user_by_id(pool, domain, user_id).await?;

        Ok(user)
    }

    pub async fn get_user_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let row = sqlx::query(
            r#"
            SELECT u.id, u.username, u.email, u.first_name, u.last_name, 
                   u.is_active, u.is_verified, u.role_id, u.last_login_at, 
                   u.created_at, u.updated_at
            FROM users u WHERE u.id = $1
            "#,
        )
        .bind(user_id)
        .fetch_one(&pg_pool)
        .await?;

        let id: Uuid = row.get("id");
        let username: String = row.get("username");
        let email: String = row.get("email");
        let first_name: Option<String> = row.get("first_name");
        let last_name: Option<String> = row.get("last_name");
        let is_active: bool = row.get("is_active");
        let is_verified: bool = row.get("is_verified");
        let role_id: Option<Uuid> = row.get("role_id");
        let last_login_at: Option<DateTime<Utc>> = row.get("last_login_at");
        let created_at: DateTime<Utc> = row.get("created_at");
        let updated_at: DateTime<Utc> = row.get("updated_at");

        // Role system simplified for now
        let role = None;

        Ok(UserResponse {
            id,
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
        page: Option<u32>,
        limit: Option<u32>,
    ) -> Result<Vec<UserResponse>, Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = ((page - 1) * limit) as i64;

        let rows = sqlx::query(
            r#"
            SELECT u.id, u.username, u.email, u.first_name, u.last_name, 
                   u.is_active, u.is_verified, u.role_id, u.last_login_at, 
                   u.created_at, u.updated_at
            FROM users u 
            ORDER BY u.created_at DESC 
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset)
        .fetch_all(&pg_pool)
        .await?;

        let mut users = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let username: String = row.get("username");
            let email: String = row.get("email");
            let first_name: Option<String> = row.get("first_name");
            let last_name: Option<String> = row.get("last_name");
            let is_active: bool = row.get("is_active");
            let is_verified: bool = row.get("is_verified");
            let role_id: Option<Uuid> = row.get("role_id");
            let last_login_at: Option<DateTime<Utc>> = row.get("last_login_at");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            // Role system simplified for now
            let role = None;

            users.push(UserResponse {
                id,
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
            });
        }

        Ok(users)
    }

    pub async fn update_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
        request: UpdateUserRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let now = Utc::now();

        // Build dynamic update query
        let mut updates = vec!["updated_at = $1".to_string()];
        let mut bind_index = 2;

        if request.username.is_some() {
            updates.push(format!("username = ${}", bind_index));
            bind_index += 1;
        }
        if request.email.is_some() {
            updates.push(format!("email = ${}", bind_index));
            bind_index += 1;
        }
        if request.first_name.is_some() {
            updates.push(format!("first_name = ${}", bind_index));
            bind_index += 1;
        }
        if request.last_name.is_some() {
            updates.push(format!("last_name = ${}", bind_index));
            bind_index += 1;
        }
        if request.is_active.is_some() {
            updates.push(format!("is_active = ${}", bind_index));
            bind_index += 1;
        }
        // is_verified field removed for simplicity
        if request.role_id.is_some() {
            updates.push(format!("role_id = ${}", bind_index));
            bind_index += 1;
        }

        let query = format!(
            "UPDATE users SET {} WHERE id = ${}",
            updates.join(", "),
            bind_index
        );

        let mut query_builder = sqlx::query(&query).bind(now);

        if let Some(username) = &request.username {
            query_builder = query_builder.bind(username);
        }
        if let Some(email) = &request.email {
            query_builder = query_builder.bind(email);
        }
        if let Some(first_name) = &request.first_name {
            query_builder = query_builder.bind(first_name);
        }
        if let Some(last_name) = &request.last_name {
            query_builder = query_builder.bind(last_name);
        }
        if let Some(is_active) = request.is_active {
            query_builder = query_builder.bind(is_active);
        }
        // is_verified field removed for simplicity
        if let Some(role_id) = request.role_id {
            query_builder = query_builder.bind(role_id);
        }

        query_builder = query_builder.bind(user_id);
        query_builder.execute(&pg_pool).await?;

        // Get the updated user
        self.get_user_by_id(pool, domain, user_id).await
    }

    pub async fn delete_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&pg_pool)
            .await?;

        Ok(())
    }

    pub async fn authenticate_user(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: LoginRequest,
    ) -> Result<UserResponse, Box<dyn std::error::Error>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        // Get user by username or email
        let row = sqlx::query(
            r#"
            SELECT id, password_hash FROM users 
            WHERE (username = $1 OR email = $1) AND is_active = true
            "#,
        )
        .bind(&request.username_or_email)
        .fetch_one(&pg_pool)
        .await?;

        let user_id: Uuid = row.get("id");
        let password_hash: String = row.get("password_hash");

        // Verify password
        if !verify(&request.password, &password_hash)? {
            return Err("Invalid credentials".into());
        }

        // Update last login time
        let now = Utc::now();
        sqlx::query("UPDATE users SET last_login_at = $1, updated_at = $2 WHERE id = $3")
            .bind(now)
            .bind(now)
            .bind(user_id)
            .execute(&pg_pool)
            .await?;

        // Get the user details
        self.get_user_by_id(pool, domain, user_id).await
    }

    // Role functionality removed for simplicity
}
