use clickhouse::Client;

pub async fn run_base_migrations(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
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

    println!("Base database migrations completed successfully");
    Ok(())
}
