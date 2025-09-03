use clickhouse::Client;
use std::env;
use url::Url;

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

        // Use port 8123 for HTTP instead of 443 for HTTPS
        let http_port = if port == 443 { 8123 } else { port };

        let http_url = if username.is_empty() {
            format!("http://{}:{}/{}", host, http_port, database)
        } else {
            format!(
                "http://{}:{}@{}:{}/{}",
                username, password, host, http_port, database
            )
        };

        println!("⚠️  Converted HTTPS URL to HTTP for ClickHouse client compatibility");
        println!("   Original: {}", url_str);
        println!("   Converted: {}", http_url);

        Ok(http_url)
    } else {
        // Already HTTP, return as-is
        Ok(url_str.to_string())
    }
}

// Inline migration functions since we can't import from external crate in binary
async fn run_base_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Create client_connect table
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

    // Create base user table
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

    println!("✅ Base migrations completed");
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
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
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
                description Nullable(String),
                resource String,
                action String,
                is_active Bool DEFAULT true,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create blog table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog (
                id String,
                title String,
                content String,
                excerpt String,
                slug String,
                author_id String,
                status String DEFAULT 'draft',
                published_at Nullable(DateTime),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create tag table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS tag (
                id String,
                name String,
                description Nullable(String),
                slug String,
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create blog_category table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_category (
                id String,
                name String,
                description Nullable(String),
                slug String,
                parent_id Nullable(String),
                created_at DateTime DEFAULT now(),
                updated_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create auth_tracking table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS auth_tracking (
                id String,
                user_id String,
                event_type String,
                ip_address String,
                user_agent String,
                success Bool,
                details Nullable(String),
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    // Create blog_tracking table
    client
        .query(
            r#"
            CREATE TABLE IF NOT EXISTS blog_tracking (
                id String,
                blog_id String,
                user_id Nullable(String),
                event_type String,
                ip_address String,
                user_agent String,
                details Nullable(String),
                created_at DateTime DEFAULT now()
            ) ENGINE = MergeTree()
            ORDER BY id
            "#,
        )
        .execute()
        .await?;

    println!("✅ Client migrations completed");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load environment variables
    dotenvy::dotenv().ok();

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return Ok(());
    }

    let command = &args[1];

    match command.as_str() {
        "base" => {
            println!("🚀 Running Base Database Migrations...");
            run_base_migration().await?;
            println!("✅ Base migration completed successfully!");
        }
        "client" => {
            if args.len() < 3 {
                println!("❌ Client migration requires database URL or domain");
                println!(
                    "Usage: migrate client <database_url> OR migrate client --domain <domain>"
                );
                return Ok(());
            }

            if args[2] == "--domain" {
                if args.len() < 4 {
                    println!("❌ Domain name required");
                    return Ok(());
                }
                let domain = &args[3];
                println!(
                    "🚀 Running Client Database Migrations for domain: {}",
                    domain
                );
                run_client_migration_by_domain(domain).await?;
            } else {
                let database_url = &args[2];
                println!("🚀 Running Client Database Migrations...");
                run_client_migration_by_url(database_url).await?;
            }
            println!("✅ Client migration completed successfully!");
        }
        "all" => {
            println!("🚀 Running All Migrations...");

            // Run base migration first
            println!("📊 Running base migration...");
            run_base_migration().await?;

            // Run all client migrations
            println!("📊 Running client migrations for all domains...");
            run_all_client_migrations().await?;

            println!("✅ All migrations completed successfully!");
        }
        "help" | "--help" | "-h" => {
            print_help();
        }
        _ => {
            println!("❌ Unknown command: {}", command);
            print_usage();
        }
    }

    Ok(())
}

async fn run_base_migration() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "https://clickhouse:vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg@clickhouse-production-71f9.up.railway.app:443/railway".to_string());

    println!("Original URL: {}", base_url);

    // Convert HTTPS to HTTP for ClickHouse client compatibility
    let converted_url = convert_clickhouse_url(&base_url)?;
    println!("Converted URL: {}", converted_url);

    let base_client = Client::default().with_url(converted_url);
    run_base_migrations(&base_client).await?;
    Ok(())
}

async fn run_client_migration_by_url(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::default().with_url(database_url);
    run_client_migrations(&client).await?;
    Ok(())
}

async fn run_client_migration_by_domain(domain: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Get database URL for domain from base database
    let base_url = env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "https://clickhouse:vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg@clickhouse-production-71f9.up.railway.app:443/railway".to_string());

    let converted_base_url = convert_clickhouse_url(&base_url)?;
    let base_client = Client::default().with_url(converted_base_url);

    let query = "SELECT database_url FROM client_connect WHERE domain = ? AND is_active = true";
    let database_url = base_client
        .query(query)
        .bind(domain)
        .fetch_one::<String>()
        .await?;
    println!(
        "📊 Found database URL for domain '{}': {}",
        domain, database_url
    );

    run_client_migration_by_url(&database_url).await?;
    Ok(())
}

async fn run_all_client_migrations() -> Result<(), Box<dyn std::error::Error>> {
    let base_url = env::var("CLICKHOUSE_URL")
        .unwrap_or_else(|_| "https://clickhouse:vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg@clickhouse-production-71f9.up.railway.app:443/railway".to_string());

    let converted_base_url = convert_clickhouse_url(&base_url)?;
    let base_client = Client::default().with_url(converted_base_url);

    let query = "SELECT domain, database_url FROM client_connect WHERE is_active = true";
    let rows = base_client
        .query(query)
        .fetch_all::<(String, String)>()
        .await?;

    if rows.is_empty() {
        println!("ℹ️  No active client connections found");
        return Ok(());
    }

    println!("📊 Found {} active client connection(s)", rows.len());

    for row in rows {
        let domain = row.0;
        let database_url = row.1;

        println!("🔄 Migrating domain: {}", domain);
        match run_client_migration_by_url(&database_url).await {
            Ok(()) => println!("  ✅ Migration successful for {}", domain),
            Err(e) => println!("  ❌ Migration failed for {}: {}", domain, e),
        }
    }

    Ok(())
}

fn print_usage() {
    println!("Usage: migrate <command> [options]");
    println!();
    println!("Commands:");
    println!("  base                     Run base database migrations");
    println!("  client <database_url>    Run client migrations for specific database URL");
    println!("  client --domain <domain> Run client migrations for specific domain");
    println!("  all                      Run all migrations (base + all clients)");
    println!("  help                     Show this help message");
}

fn print_help() {
    println!("🗃️  Database Migration Tool");
    println!("==========================");
    println!();
    print_usage();
    println!();
    println!("Examples:");
    println!("  # Run base database migrations");
    println!("  migrate base");
    println!();
    println!("  # Run client migrations for specific URL");
    println!("  migrate client \"https://user:pass@host:port/db\"");
    println!();
    println!("  # Run client migrations for specific domain");
    println!("  migrate client --domain example.com");
    println!();
    println!("  # Run all migrations");
    println!("  migrate all");
    println!();
    println!("Environment Variables:");
    println!("  CLICKHOUSE_URL    Base ClickHouse database URL");
    println!("  DATABASE_NAME     Base database name");
    println!();
    println!("Note: Make sure to set the environment variables in your .env file");
}
