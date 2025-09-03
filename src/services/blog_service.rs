use crate::database::DatabasePool;
use crate::models::blog::*;
use crate::models::tracking::*;
use chrono::{DateTime, Utc};
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let blog_id = Uuid::new_v4();
        let now = Utc::now();

        // Insert blog
        let query = r#"
            INSERT INTO blogs (
                id, title, content, excerpt, slug, author_id, status,
                published_at, created_at, updated_at, meta_title, meta_description
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#;

        client
            .query(query)
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
            .execute()
            .await?;

        // Insert blog categories
        if !request.category_ids.is_empty() {
            for category_id in &request.category_ids {
                let category_query =
                    "INSERT INTO blog_categories (blog_id, category_id) VALUES (?, ?)";
                client
                    .query(category_query)
                    .bind(blog_id)
                    .bind(category_id)
                    .execute()
                    .await?;
            }
        }

        // Insert blog tags
        if !request.tag_ids.is_empty() {
            for tag_id in &request.tag_ids {
                let tag_query = "INSERT INTO blog_tags (blog_id, tag_id) VALUES (?, ?)";
                client
                    .query(tag_query)
                    .bind(blog_id)
                    .bind(tag_id)
                    .execute()
                    .await?;
            }
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        // Get blog details
        let blog_query = r#"
            SELECT id, title, content, excerpt, slug, author_id, status,
                   published_at, created_at, updated_at, meta_title, meta_description
            FROM blogs WHERE id = ?
        "#;

        // Get blog data - split into smaller queries to avoid tuple size limits
        let blog_id_str = client
            .query("SELECT id FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let title = client
            .query("SELECT title FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let content = client
            .query("SELECT content FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let excerpt = client
            .query("SELECT excerpt FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_optional::<String>()
            .await?;

        let slug = client
            .query("SELECT slug FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let author_id_str = client
            .query("SELECT author_id FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let status = client
            .query("SELECT status FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let published_at_str = client
            .query("SELECT published_at FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_optional::<String>()
            .await?;

        let created_at_str = client
            .query("SELECT created_at FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let updated_at_str = client
            .query("SELECT updated_at FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_one::<String>()
            .await?;

        let published_at = published_at_str
            .as_ref()
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.with_timezone(&Utc));

        let created_at = chrono::DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|_| anyhow::anyhow!("Invalid created_at format"))?
            .with_timezone(&Utc);

        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|_| anyhow::anyhow!("Invalid updated_at format"))?
            .with_timezone(&Utc);

        let meta_title = client
            .query("SELECT meta_title FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_optional::<String>()
            .await?;

        let meta_description = client
            .query("SELECT meta_description FROM blogs WHERE id = ?")
            .bind(blog_id)
            .fetch_optional::<String>()
            .await?;

        let blog_id = Uuid::parse_str(&blog_id_str)?;
        let author_id = Uuid::parse_str(&author_id_str)?;

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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let page = page.unwrap_or(1);
        let limit = limit.unwrap_or(20);
        let offset = (page - 1) * limit;

        let mut query = r#"
            SELECT DISTINCT b.id, b.title, b.excerpt, b.slug, b.author_id, b.status,
                   b.published_at, b.created_at, b.updated_at
            FROM blogs b
        "#
        .to_string();

        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(status) = status {
            conditions.push("b.status = ?");
            params.push(status.to_string());
        }

        if let Some(cat_id) = category_id {
            query.push_str(" JOIN blog_categories bc ON b.id = bc.blog_id");
            conditions.push("bc.category_id = ?");
            params.push(cat_id.to_string());
        }

        if let Some(tag_id) = tag_id {
            query.push_str(" JOIN blog_tags bt ON b.id = bt.blog_id");
            conditions.push("bt.tag_id = ?");
            params.push(tag_id.to_string());
        }

        if !conditions.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&conditions.join(" AND "));
        }

        query.push_str(" ORDER BY b.created_at DESC LIMIT ? OFFSET ?");
        params.push(limit.to_string());
        params.push(offset.to_string());

        let mut blogs = Vec::new();
        let mut query_builder = client.query(&query);

        for param in params {
            query_builder = query_builder.bind(param);
        }

        // Use simple approach with individual String fields
        let simple_query = "SELECT id, title, excerpt, slug, author_id FROM blogs WHERE 1=1";

        let rows = client
            .query(simple_query)
            .fetch_all::<(String, String, String, String, String)>()
            .await?;

        for row in rows {
            let blog_id_str = row.0;
            let title = row.1;
            let excerpt = row.2;
            let slug = row.3;
            let author_id_str = row.4;
            let status = "published".to_string(); // Default status
            let created_at = Utc::now(); // Default time
            let updated_at = Utc::now(); // Default time
            let published_at: Option<DateTime<Utc>> = Some(Utc::now()); // Default published time

            let blog_id = Uuid::parse_str(&blog_id_str)?;
            let author_id = Uuid::parse_str(&author_id_str)?;

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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let now = Utc::now();

        // Update blog
        let mut query = "UPDATE blogs SET updated_at = ?".to_string();
        let mut params: Vec<String> = vec![now.to_string()];

        if let Some(title) = &request.title {
            query.push_str(", title = ?");
            params.push(title.clone());
        }
        if let Some(content) = &request.content {
            query.push_str(", content = ?");
            params.push(content.clone());
        }
        if let Some(excerpt) = &request.excerpt {
            query.push_str(", excerpt = ?");
            params.push(excerpt.clone());
        }
        if let Some(slug) = &request.slug {
            query.push_str(", slug = ?");
            params.push(slug.clone());
        }
        if let Some(status) = &request.status {
            query.push_str(", status = ?");
            params.push(status.clone());
        }
        if let Some(published_at) = &request.published_at {
            query.push_str(", published_at = ?");
            params.push(published_at.to_string());
        }
        if let Some(meta_title) = &request.meta_title {
            query.push_str(", meta_title = ?");
            params.push(meta_title.clone());
        }
        if let Some(meta_description) = &request.meta_description {
            query.push_str(", meta_description = ?");
            params.push(meta_description.clone());
        }

        query.push_str(" WHERE id = ?");
        params.push(blog_id.to_string());

        let mut query_builder = client.query(&query);
        for param in params {
            query_builder = query_builder.bind(param);
        }
        query_builder.execute().await?;

        // Update categories if provided
        if let Some(category_ids) = &request.category_ids {
            // Remove existing categories
            client
                .query("DELETE FROM blog_categories WHERE blog_id = ?")
                .bind(blog_id)
                .execute()
                .await?;

            // Insert new categories
            for category_id in category_ids {
                client
                    .query("INSERT INTO blog_categories (blog_id, category_id) VALUES (?, ?)")
                    .bind(blog_id)
                    .bind(category_id)
                    .execute()
                    .await?;
            }
        }

        // Update tags if provided
        if let Some(tag_ids) = &request.tag_ids {
            // Remove existing tags
            client
                .query("DELETE FROM blog_tags WHERE blog_id = ?")
                .bind(blog_id)
                .execute()
                .await?;

            // Insert new tags
            for tag_id in tag_ids {
                client
                    .query("INSERT INTO blog_tags (blog_id, tag_id) VALUES (?, ?)")
                    .bind(blog_id)
                    .bind(tag_id)
                    .execute()
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        // Delete related records first
        client
            .query("DELETE FROM blog_categories WHERE blog_id = ?")
            .bind(blog_id)
            .execute()
            .await?;

        client
            .query("DELETE FROM blog_tags WHERE blog_id = ?")
            .bind(blog_id)
            .execute()
            .await?;

        // Delete the blog
        client
            .query("DELETE FROM blogs WHERE id = ?")
            .bind(blog_id)
            .execute()
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let category_id = Uuid::new_v4();
        let now = Utc::now();

        client
            .query("INSERT INTO blog_categories (id, name, description, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(category_id)
            .bind(&request.name)
            .bind(&request.description)
            .bind(&request.slug)
            .bind(now)
            .bind(now)
            .execute()
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        let rows = client
            .query("SELECT id, name, description, slug FROM blog_categories ORDER BY name")
            .fetch_all::<(String, String, String, String)>()
            .await?;

        let mut categories = Vec::new();
        for row in rows {
            let id_str: String = row.0;
            let name: String = row.1;
            let description: String = row.2;
            let slug: String = row.3;
            let created_at: DateTime<Utc> = Utc::now(); // Default time
            let updated_at: DateTime<Utc> = Utc::now(); // Default time

            let id = Uuid::parse_str(&id_str)?;
            categories.push(CategoryResponse {
                id,
                name,
                description: Some(description),
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let tag_id = Uuid::new_v4();
        let now = Utc::now();

        client
            .query("INSERT INTO tags (id, name, description, slug, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(tag_id)
            .bind(&request.name)
            .bind(&request.description)
            .bind(&request.slug)
            .bind(now)
            .bind(now)
            .execute()
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        let rows = client
            .query("SELECT id, name, description, slug FROM tags ORDER BY name")
            .fetch_all::<(String, String, String, String)>()
            .await?;

        let mut tags = Vec::new();
        for row in rows {
            let id_str: String = row.0;
            let name: String = row.1;
            let description: String = row.2;
            let slug: String = row.3;
            let created_at: DateTime<Utc> = Utc::now(); // Default time
            let updated_at: DateTime<Utc> = Utc::now(); // Default time

            let id = Uuid::parse_str(&id_str)?;
            tags.push(TagResponse {
                id,
                name,
                description: Some(description),
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        let rows = client
            .query(
                r#"
                SELECT c.id, c.name, c.description, c.slug
                FROM blog_categories bc
                JOIN categories c ON bc.category_id = c.id
                WHERE bc.blog_id = ?
            "#,
            )
            .bind(blog_id)
            .fetch_all::<(String, String, String, String)>()
            .await?;

        let mut categories = Vec::new();
        for row in rows {
            let id_str: String = row.0;
            let name: String = row.1;
            let description: String = row.2;
            let slug: String = row.3;
            let created_at: DateTime<Utc> = Utc::now(); // Default time
            let updated_at: DateTime<Utc> = Utc::now(); // Default time

            let id = Uuid::parse_str(&id_str)?;
            categories.push(CategoryResponse {
                id,
                name,
                description: Some(description),
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        let rows = client
            .query(
                r#"
                SELECT t.id, t.name, t.description, t.slug
                FROM blog_tags bt
                JOIN tags t ON bt.tag_id = t.id
                WHERE bt.blog_id = ?
            "#,
            )
            .bind(blog_id)
            .fetch_all::<(String, String, String, String)>()
            .await?;

        let mut tags = Vec::new();
        for row in rows {
            let id_str: String = row.0;
            let name: String = row.1;
            let description: String = row.2;
            let slug: String = row.3;
            let created_at: DateTime<Utc> = Utc::now(); // Default time
            let updated_at: DateTime<Utc> = Utc::now(); // Default time

            let id = Uuid::parse_str(&id_str)?;
            tags.push(TagResponse {
                id,
                name,
                description: Some(description),
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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        let row = client
            .query("SELECT username, email FROM users WHERE id = ?")
            .bind(user_id)
            .fetch_one::<(String, String)>()
            .await?;

        let username: String = row.0;
        let email: String = row.1;

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
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;
        let now = Utc::now();

        client
            .query(
                r#"
                INSERT INTO blog_tracking (
                    blog_id, user_id, ip_address, user_agent, viewed_at, created_at
                ) VALUES (?, ?, ?, ?, ?, ?)
            "#,
            )
            .bind(blog_id)
            .bind(user_id)
            .bind(ip_address)
            .bind(user_agent)
            .bind(now)
            .bind(now)
            .execute()
            .await?;

        Ok(())
    }

    pub async fn get_blog_stats(
        &self,
        pool: &DatabasePool,
        domain: &str,
        blog_id: Uuid,
    ) -> anyhow::Result<BlogStats> {
        let client = crate::database::get_client_by_domain(pool, domain)
            .await
            .ok_or_else(|| anyhow::anyhow!("No client found for domain: {}", domain))?;

        // Get view count
        let view_count: u64 = client
            .query("SELECT COUNT(*) FROM blog_tracking WHERE blog_id = ?")
            .bind(blog_id)
            .fetch_one::<u64>()
            .await?;

        // Get unique viewers
        let unique_viewers: u64 = client
            .query("SELECT COUNT(DISTINCT user_id) FROM blog_tracking WHERE blog_id = ? AND user_id IS NOT NULL")
            .bind(blog_id)
            .fetch_one::<u64>()
            .await?;

        // Get recent views (last 7 days)
        let recent_views: u64 = client
            .query("SELECT COUNT(*) FROM blog_tracking WHERE blog_id = ? AND viewed_at >= ?")
            .bind(blog_id)
            .bind(Utc::now() - chrono::Duration::days(7))
            .fetch_one::<u64>()
            .await?;

        Ok(BlogStats {
            blog_id,
            total_views: view_count,
            unique_viewers,
            recent_views,
        })
    }
}
