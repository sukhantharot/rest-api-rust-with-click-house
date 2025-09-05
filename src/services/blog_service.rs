use crate::database::DatabasePool;
use crate::models::blog::*;
use chrono::{DateTime, Utc};
use sqlx::Row;
use uuid::Uuid;

pub struct BlogService;

impl BlogService {
    pub fn new() -> Self {
        Self
    }

    // Blog CRUD operations
    pub async fn create_blog(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateBlogRequest,
        author_id: Uuid,
    ) -> anyhow::Result<BlogResponse> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let blog_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert blog
        sqlx::query(
            r#"
            INSERT INTO blogs (
                id, title, content, excerpt, slug, author_id, status,
                published_at, created_at, updated_at, meta_title, meta_description
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(blog_id)
        .bind(&request.title)
        .bind(&request.content)
        .bind(&request.excerpt)
        .bind(&request.slug)
        .bind(author_id)
        .bind(&request.status)
        .bind(request.published_at)
        .bind(now)
        .bind(now)
        .bind(&request.meta_title)
        .bind(&request.meta_description)
        .execute(&pg_pool)
        .await?;

        // Insert blog categories
        for category_id in &request.category_ids {
            let category_relation_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO blog_categories (id, blog_id, category_id) VALUES ($1, $2, $3)",
            )
            .bind(category_relation_id)
            .bind(blog_id)
            .bind(category_id)
            .execute(&pg_pool)
            .await?;
        }

        // Insert blog tags
        for tag_id in &request.tag_ids {
            let tag_relation_id = Uuid::new_v4();
            sqlx::query("INSERT INTO blog_tags (id, blog_id, tag_id) VALUES ($1, $2, $3)")
                .bind(tag_relation_id)
                .bind(blog_id)
                .bind(tag_id)
                .execute(&pg_pool)
                .await?;
        }

        // Get the created blog with full details
        self.get_blog_by_id(pool, domain, blog_id).await
    }

    pub async fn get_blog_by_id(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<BlogResponse> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        // Get blog details
        let blog_row = sqlx::query(
            r#"
            SELECT id, title, content, excerpt, slug, author_id, status,
                   published_at, created_at, updated_at, meta_title, meta_description
            FROM blogs WHERE id = $1
            "#,
        )
        .bind(blog_id)
        .fetch_one(&pg_pool)
        .await?;

        let blog_id: Uuid = blog_row.get("id");
        let title: String = blog_row.get("title");
        let content: String = blog_row.get("content");
        let excerpt: Option<String> = blog_row.get("excerpt");
        let slug: String = blog_row.get("slug");
        let author_id: Uuid = blog_row.get("author_id");
        let status: String = blog_row.get("status");
        let published_at: Option<DateTime<Utc>> = blog_row.get("published_at");
        let created_at: DateTime<Utc> = blog_row.get("created_at");
        let updated_at: DateTime<Utc> = blog_row.get("updated_at");
        let meta_title: Option<String> = blog_row.get("meta_title");
        let meta_description: Option<String> = blog_row.get("meta_description");

        // Get categories
        let categories = self.get_blog_categories(pool, domain, blog_id).await?;

        // Get tags
        let tags = self.get_blog_tags(pool, domain, blog_id).await?;

        // Get author info
        let author = self.get_user_info(pool, domain, author_id).await?;

        Ok(BlogResponse {
            id: blog_id,
            title,
            content,
            excerpt,
            slug,
            author,
            status,
            published_at,
            created_at,
            updated_at,
            meta_title,
            meta_description,
            categories,
            tags,
        })
    }

    pub async fn get_blogs(
        &self,
        pool: &DatabasePool,
        domain: &str,
        page: Option<u32>,
        limit: Option<u32>,
        status: Option<&str>,
        category_id: Option<Uuid>,
        tag_id: Option<Uuid>,
    ) -> anyhow::Result<Vec<BlogResponse>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = ((page - 1) * limit) as i64;

        let mut query = r#"
            SELECT DISTINCT b.id, b.title, b.excerpt, b.slug, b.author_id, b.status,
                   b.published_at, b.created_at, b.updated_at
            FROM blogs b
        "#
        .to_string();

        let mut conditions = Vec::new();
        let mut bind_index = 1;

        if status.is_some() {
            query.push_str(" WHERE b.status = $");
            query.push_str(&bind_index.to_string());
            bind_index += 1;
            conditions.push("status");
        }

        if category_id.is_some() {
            query.push_str(" JOIN blog_categories bc ON b.id = bc.blog_id");
            let connector = if conditions.is_empty() {
                " WHERE"
            } else {
                " AND"
            };
            query.push_str(connector);
            query.push_str(" bc.category_id = $");
            query.push_str(&bind_index.to_string());
            bind_index += 1;
            conditions.push("category");
        }

        if tag_id.is_some() {
            query.push_str(" JOIN blog_tags bt ON b.id = bt.blog_id");
            let connector = if conditions.is_empty() {
                " WHERE"
            } else {
                " AND"
            };
            query.push_str(connector);
            query.push_str(" bt.tag_id = $");
            query.push_str(&bind_index.to_string());
            bind_index += 1;
            conditions.push("tag");
        }

        query.push_str(" ORDER BY b.created_at DESC LIMIT $");
        query.push_str(&bind_index.to_string());
        bind_index += 1;
        query.push_str(" OFFSET $");
        query.push_str(&bind_index.to_string());

        let mut query_builder = sqlx::query(&query);

        if let Some(status) = status {
            query_builder = query_builder.bind(status);
        }
        if let Some(cat_id) = category_id {
            query_builder = query_builder.bind(cat_id);
        }
        if let Some(tag_id) = tag_id {
            query_builder = query_builder.bind(tag_id);
        }
        query_builder = query_builder.bind(limit as i64);
        query_builder = query_builder.bind(offset);

        let rows = query_builder.fetch_all(&pg_pool).await?;

        let mut blogs = Vec::new();
        for row in rows {
            let blog_id: Uuid = row.get("id");
            // Get full blog details
            let blog = self.get_blog_by_id(pool, domain, blog_id).await?;
            blogs.push(blog);
        }

        Ok(blogs)
    }

    pub async fn update_blog(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
        request: UpdateBlogRequest,
    ) -> anyhow::Result<BlogResponse> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let now = Utc::now();

        // Build dynamic update query
        let mut updates = vec!["updated_at = $1".to_string()];
        let mut bind_index = 2;

        if request.title.is_some() {
            updates.push(format!("title = ${}", bind_index));
            bind_index += 1;
        }
        if request.content.is_some() {
            updates.push(format!("content = ${}", bind_index));
            bind_index += 1;
        }
        if request.excerpt.is_some() {
            updates.push(format!("excerpt = ${}", bind_index));
            bind_index += 1;
        }
        if request.slug.is_some() {
            updates.push(format!("slug = ${}", bind_index));
            bind_index += 1;
        }
        if request.status.is_some() {
            updates.push(format!("status = ${}", bind_index));
            bind_index += 1;
        }
        if request.published_at.is_some() {
            updates.push(format!("published_at = ${}", bind_index));
            bind_index += 1;
        }
        if request.meta_title.is_some() {
            updates.push(format!("meta_title = ${}", bind_index));
            bind_index += 1;
        }
        if request.meta_description.is_some() {
            updates.push(format!("meta_description = ${}", bind_index));
            bind_index += 1;
        }

        let query = format!(
            "UPDATE blogs SET {} WHERE id = ${}",
            updates.join(", "),
            bind_index
        );

        let mut query_builder = sqlx::query(&query).bind(now);

        if let Some(title) = &request.title {
            query_builder = query_builder.bind(title);
        }
        if let Some(content) = &request.content {
            query_builder = query_builder.bind(content);
        }
        if let Some(excerpt) = &request.excerpt {
            query_builder = query_builder.bind(excerpt);
        }
        if let Some(slug) = &request.slug {
            query_builder = query_builder.bind(slug);
        }
        if let Some(status) = &request.status {
            query_builder = query_builder.bind(status);
        }
        if let Some(published_at) = &request.published_at {
            query_builder = query_builder.bind(published_at);
        }
        if let Some(meta_title) = &request.meta_title {
            query_builder = query_builder.bind(meta_title);
        }
        if let Some(meta_description) = &request.meta_description {
            query_builder = query_builder.bind(meta_description);
        }

        query_builder = query_builder.bind(blog_id);
        query_builder.execute(&pg_pool).await?;

        // Update categories if provided
        if let Some(category_ids) = &request.category_ids {
            // Remove existing categories
            sqlx::query("DELETE FROM blog_categories WHERE blog_id = $1")
                .bind(blog_id)
                .execute(&pg_pool)
                .await?;

            // Insert new categories
            for category_id in category_ids {
                let relation_id = Uuid::new_v4();
                sqlx::query(
                    "INSERT INTO blog_categories (id, blog_id, category_id) VALUES ($1, $2, $3)",
                )
                .bind(relation_id)
                .bind(blog_id)
                .bind(category_id)
                .execute(&pg_pool)
                .await?;
            }
        }

        // Update tags if provided
        if let Some(tag_ids) = &request.tag_ids {
            // Remove existing tags
            sqlx::query("DELETE FROM blog_tags WHERE blog_id = $1")
                .bind(blog_id)
                .execute(&pg_pool)
                .await?;

            // Insert new tags
            for tag_id in tag_ids {
                let relation_id = Uuid::new_v4();
                sqlx::query("INSERT INTO blog_tags (id, blog_id, tag_id) VALUES ($1, $2, $3)")
                    .bind(relation_id)
                    .bind(blog_id)
                    .bind(tag_id)
                    .execute(&pg_pool)
                    .await?;
            }
        }

        // Get the updated blog
        self.get_blog_by_id(pool, domain, blog_id).await
    }

    pub async fn delete_blog(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<()> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        // PostgreSQL will handle CASCADE deletes for related records
        sqlx::query("DELETE FROM blogs WHERE id = $1")
            .bind(blog_id)
            .execute(&pg_pool)
            .await?;

        Ok(())
    }

    // Category operations
    pub async fn create_category(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateCategoryRequest,
    ) -> anyhow::Result<CategoryResponse> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let category_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO categories (id, name, description, slug, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(category_id)
        .bind(&request.name)
        .bind(&request.description)
        .bind(&request.slug)
        .bind(now)
        .bind(now)
        .execute(&pg_pool)
        .await?;

        Ok(CategoryResponse {
            id: category_id,
            name: request.name,
            description: request.description,
            slug: request.slug,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_categories(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> anyhow::Result<Vec<CategoryResponse>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let rows = sqlx::query(
            "SELECT id, name, description, slug, created_at, updated_at FROM categories ORDER BY name"
        )
        .fetch_all(&pg_pool)
        .await?;

        let mut categories = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description: Option<String> = row.get("description");
            let slug: String = row.get("slug");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            categories.push(CategoryResponse {
                id,
                name,
                description,
                slug,
                created_at,
                updated_at,
            });
        }

        Ok(categories)
    }

    // Tag operations
    pub async fn create_tag(
        &self,
        pool: &DatabasePool,
        domain: &str,
        request: CreateTagRequest,
    ) -> anyhow::Result<TagResponse> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let tag_id = Uuid::new_v4();
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO tags (id, name, description, slug, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6)"
        )
        .bind(tag_id)
        .bind(&request.name)
        .bind(&request.description)
        .bind(&request.slug)
        .bind(now)
        .bind(now)
        .execute(&pg_pool)
        .await?;

        Ok(TagResponse {
            id: tag_id,
            name: request.name,
            description: request.description,
            slug: request.slug,
            created_at: now,
            updated_at: now,
        })
    }

    pub async fn get_tags(
        &self,
        pool: &DatabasePool,
        domain: &str,
    ) -> anyhow::Result<Vec<TagResponse>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let rows = sqlx::query(
            "SELECT id, name, description, slug, created_at, updated_at FROM tags ORDER BY name",
        )
        .fetch_all(&pg_pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description: Option<String> = row.get("description");
            let slug: String = row.get("slug");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            tags.push(TagResponse {
                id,
                name,
                description,
                slug,
                created_at,
                updated_at,
            });
        }

        Ok(tags)
    }

    // Helper methods
    async fn get_blog_categories(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<Vec<CategoryResponse>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let rows = sqlx::query(
            r#"
            SELECT c.id, c.name, c.description, c.slug, c.created_at, c.updated_at
            FROM blog_categories bc
            JOIN categories c ON bc.category_id = c.id
            WHERE bc.blog_id = $1
            "#,
        )
        .bind(blog_id)
        .fetch_all(&pg_pool)
        .await?;

        let mut categories = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description: Option<String> = row.get("description");
            let slug: String = row.get("slug");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            categories.push(CategoryResponse {
                id,
                name,
                description,
                slug,
                created_at,
                updated_at,
            });
        }

        Ok(categories)
    }

    async fn get_blog_tags(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<Vec<TagResponse>> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let rows = sqlx::query(
            r#"
            SELECT t.id, t.name, t.description, t.slug, t.created_at, t.updated_at
            FROM blog_tags bt
            JOIN tags t ON bt.tag_id = t.id
            WHERE bt.blog_id = $1
            "#,
        )
        .bind(blog_id)
        .fetch_all(&pg_pool)
        .await?;

        let mut tags = Vec::new();
        for row in rows {
            let id: Uuid = row.get("id");
            let name: String = row.get("name");
            let description: Option<String> = row.get("description");
            let slug: String = row.get("slug");
            let created_at: DateTime<Utc> = row.get("created_at");
            let updated_at: DateTime<Utc> = row.get("updated_at");

            tags.push(TagResponse {
                id,
                name,
                description,
                slug,
                created_at,
                updated_at,
            });
        }

        Ok(tags)
    }

    async fn get_user_info(
        &self,
        pool: &DatabasePool,
        domain: &str,
        user_id: Uuid,
    ) -> anyhow::Result<UserInfo> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        let row = sqlx::query("SELECT username, email FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&pg_pool)
            .await?;

        let username: String = row.get("username");
        let email: String = row.get("email");

        Ok(UserInfo {
            id: user_id,
            username,
            email,
        })
    }

    // Blog tracking
    pub async fn track_blog_view(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
        user_id: Option<Uuid>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> anyhow::Result<()> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;
        let now = Utc::now();
        let tracking_id = Uuid::new_v4();

        sqlx::query(
            r#"
            INSERT INTO blog_tracking (
                id, blog_id, user_id, ip_address, user_agent, viewed_at, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(tracking_id)
        .bind(blog_id)
        .bind(user_id)
        .bind(ip_address)
        .bind(user_agent)
        .bind(now)
        .bind(now)
        .execute(&pg_pool)
        .await?;

        Ok(())
    }

    pub async fn get_blog_stats(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<BlogStats> {
        let pg_pool = crate::database::get_pool_for_domain(pool, domain).await?;

        // Get view count
        let view_count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM blog_tracking WHERE blog_id = $1")
                .bind(blog_id)
                .fetch_one(&pg_pool)
                .await?;

        // Get unique viewers
        let unique_viewers: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT user_id) FROM blog_tracking WHERE blog_id = $1 AND user_id IS NOT NULL"
        )
        .bind(blog_id)
        .fetch_one(&pg_pool)
        .await?;

        // Get recent views (last 7 days)
        let recent_views: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM blog_tracking WHERE blog_id = $1 AND viewed_at >= $2",
        )
        .bind(blog_id)
        .bind(Utc::now() - chrono::Duration::days(7))
        .fetch_one(&pg_pool)
        .await?;

        Ok(BlogStats {
            blog_id,
            total_views: view_count as u64,
            unique_viewers: unique_viewers as u64,
            recent_views: recent_views as u64,
        })
    }
}
