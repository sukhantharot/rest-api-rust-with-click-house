use serde::{Deserialize, Serialize};
use std::env;

mod logging;
pub use logging::{log_request_metrics, LogRotationConfig, LoggingConfig, RequestMetrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub database: DatabaseConfig,
    pub jwt: JwtConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    pub password: String,
    pub max_connections: u32,
    pub min_connections: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: String,
    pub expiration_hours: i64,
}

pub async fn load_config() -> Result<Config, Box<dyn std::error::Error>> {
    // Try to load .env file if it exists
    let _ = dotenvy::dotenv();

    let config = Config {
        database: DatabaseConfig {
            url: env::var("DATABASE_URL").unwrap_or_else(|_| {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
                    env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
                    env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
                    env::var("DB_PORT").unwrap_or_else(|_| "5432".to_string()),
                    env::var("DB_NAME").unwrap_or_else(|_| "railway".to_string())
                )
            }),
            host: env::var("DB_HOST").unwrap_or_else(|_| "localhost".to_string()),
            port: env::var("DB_PORT")
                .unwrap_or_else(|_| "5432".to_string())
                .parse()
                .unwrap_or(5432),
            name: env::var("DB_NAME").unwrap_or_else(|_| "railway".to_string()),
            user: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: env::var("DB_PASSWORD").unwrap_or_else(|_| "password".to_string()),
            max_connections: env::var("DB_MAX_CONNECTIONS")
                .unwrap_or_else(|_| "10".to_string())
                .parse()
                .unwrap_or(10),
            min_connections: env::var("DB_MIN_CONNECTIONS")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .unwrap_or(1),
        },

        jwt: JwtConfig {
            secret: env::var("JWT_SECRET").unwrap_or_else(|_| {
                "your-super-secret-jwt-key-here-change-in-production".to_string()
            }),
            expiration_hours: env::var("JWT_EXPIRATION_HOURS")
                .unwrap_or_else(|_| "24".to_string())
                .parse()
                .unwrap_or(24),
        },
        logging: LoggingConfig {
            level: env::var("LOG_LEVEL").unwrap_or_else(|_| "info".to_string()),
            json_format: env::var("LOG_JSON_FORMAT")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            request_logging: env::var("LOG_REQUEST_LOGGING")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            slow_request_threshold_ms: env::var("LOG_SLOW_REQUEST_THRESHOLD_MS")
                .unwrap_or_else(|_| "1000".to_string())
                .parse()
                .unwrap_or(1000),
            file_logging: env::var("LOG_FILE_LOGGING")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            file_path: env::var("LOG_FILE_PATH").ok(),
            rotation: LogRotationConfig::default(),
            performance_metrics: env::var("LOG_PERFORMANCE_METRICS")
                .unwrap_or_else(|_| "true".to_string())
                .parse()
                .unwrap_or(true),
            targets: Default::default(),
        },
    };

    Ok(config)
}
