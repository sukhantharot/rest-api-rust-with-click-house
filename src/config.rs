use serde::{Deserialize, Serialize};
use std::env;

mod logging;
pub use logging::{LogRotationConfig, LoggingConfig, RequestMetrics, log_request_metrics};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub clickhouse: ClickHouseConfig,
    pub server: ServerConfig,
    pub jwt: JwtConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClickHouseConfig {
    pub base_url: String,
    pub base_db: String,
    pub base_host: String,
    pub base_password: String,
    pub base_user: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
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
        clickhouse: ClickHouseConfig {
            base_url: env::var("CLICKHOUSE_URL").unwrap_or_else(|_| {
                "http://clickhouse-production-71f9.up.railway.app:8123".to_string()
            }),
            base_db: env::var("CLICKHOUSE_DB").unwrap_or_else(|_| "railway".to_string()),
            base_host: env::var("CLICKHOUSE_HOST")
                .unwrap_or_else(|_| "clickhouse-production-71f9.up.railway.app".to_string()),
            base_password: env::var("CLICKHOUSE_PASSWORD")
                .unwrap_or_else(|_| "vOn8UIeaAdx3Rgz7wRYuMRlUiaHWBWhg".to_string()),
            base_user: env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "clickhouse".to_string()),
        },

        server: ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .or_else(|_| env::var("SERVER_PORT"))
                .unwrap_or_else(|_| "3000".to_string())
                .parse()
                .unwrap_or(3000),
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
