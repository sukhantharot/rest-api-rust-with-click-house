use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Logging configuration

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Enable JSON format logging
    pub json_format: bool,

    /// Enable request logging
    pub request_logging: bool,

    /// Slow request threshold in milliseconds
    pub slow_request_threshold_ms: u64,

    /// Enable file logging
    pub file_logging: bool,

    /// Log file path
    pub file_path: Option<String>,

    /// Log file rotation settings
    pub rotation: LogRotationConfig,

    /// Enable performance metrics
    pub performance_metrics: bool,

    /// Additional log targets with their levels
    pub targets: HashMap<String, String>,
}

/// Log rotation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogRotationConfig {
    /// Enable log rotation
    pub enabled: bool,

    /// Maximum file size in MB before rotation
    pub max_file_size_mb: u64,

    /// Maximum number of rotated files to keep
    pub max_files: u32,

    /// Rotation interval (daily, hourly, never)
    pub interval: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        let mut targets = HashMap::new();
        targets.insert("rest_api_rust_clickhouse".to_string(), "info".to_string());
        targets.insert("tower_http".to_string(), "info".to_string());
        targets.insert("axum".to_string(), "info".to_string());
        targets.insert("clickhouse".to_string(), "warn".to_string());

        Self {
            level: "info".to_string(),
            json_format: false,
            request_logging: true,
            slow_request_threshold_ms: 1000,
            file_logging: false,
            file_path: Some("logs/app.log".to_string()),
            rotation: LogRotationConfig::default(),
            performance_metrics: true,
            targets,
        }
    }
}

impl Default for LogRotationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_file_size_mb: 100,
            max_files: 10,
            interval: "daily".to_string(),
        }
    }
}

impl LoggingConfig {
    /// Create EnvFilter from configuration
    pub fn create_env_filter(&self) -> tracing_subscriber::EnvFilter {
        let mut filter_string = self.level.clone();

        for (target, level) in &self.targets {
            filter_string.push(',');
            filter_string.push_str(&format!("{}={}", target, level));
        }

        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| filter_string.into())
    }

    /// Get slow request threshold as Duration
    pub fn slow_request_threshold(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.slow_request_threshold_ms)
    }
}

/// Performance metrics for request logging
#[derive(Debug, Clone)]
pub struct RequestMetrics {
    pub request_id: String,
    pub method: String,
    pub path: String,
    pub status_code: u16,
    pub duration_ms: u128,
    pub client_ip: Option<String>,
    pub user_agent: Option<String>,
    pub request_size: usize,
    pub response_size: usize,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl RequestMetrics {
    pub fn new(request_id: String) -> Self {
        Self {
            request_id,
            method: String::new(),
            path: String::new(),
            status_code: 0,
            duration_ms: 0,
            client_ip: None,
            user_agent: None,
            request_size: 0,
            response_size: 0,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn is_slow(&self, threshold: std::time::Duration) -> bool {
        self.duration_ms > threshold.as_millis()
    }

    pub fn is_error(&self) -> bool {
        self.status_code >= 400
    }

    pub fn is_server_error(&self) -> bool {
        self.status_code >= 500
    }
}

/// Log request metrics in structured format
pub fn log_request_metrics(metrics: &RequestMetrics, config: &LoggingConfig) {
    let is_slow = metrics.is_slow(config.slow_request_threshold());
    let is_error = metrics.is_error();

    if is_error {
        tracing::warn!(
            request_id = %metrics.request_id,
            method = %metrics.method,
            path = %metrics.path,
            status = %metrics.status_code,
            duration_ms = %metrics.duration_ms,
            client_ip = ?metrics.client_ip,
            user_agent = ?metrics.user_agent,
            request_size = %metrics.request_size,
            response_size = %metrics.response_size,
            timestamp = %metrics.timestamp.to_rfc3339(),
            slow_request = %is_slow,
            "❌ Request completed with error"
        );
    } else if is_slow {
        tracing::warn!(
            request_id = %metrics.request_id,
            method = %metrics.method,
            path = %metrics.path,
            status = %metrics.status_code,
            duration_ms = %metrics.duration_ms,
            client_ip = ?metrics.client_ip,
            user_agent = ?metrics.user_agent,
            request_size = %metrics.request_size,
            response_size = %metrics.response_size,
            timestamp = %metrics.timestamp.to_rfc3339(),
            "🐌 Slow request completed"
        );
    } else {
        tracing::info!(
            request_id = %metrics.request_id,
            method = %metrics.method,
            path = %metrics.path,
            status = %metrics.status_code,
            duration_ms = %metrics.duration_ms,
            client_ip = ?metrics.client_ip,
            user_agent = ?metrics.user_agent,
            request_size = %metrics.request_size,
            response_size = %metrics.response_size,
            timestamp = %metrics.timestamp.to_rfc3339(),
            "✅ Request completed"
        );
    }

    // Log performance metrics if enabled
    if config.performance_metrics {
        tracing::debug!(
            request_id = %metrics.request_id,
            performance.duration_ms = %metrics.duration_ms,
            performance.request_size_bytes = %metrics.request_size,
            performance.response_size_bytes = %metrics.response_size,
            performance.requests_per_second = %(1000.0 / metrics.duration_ms as f64),
            "📊 Performance metrics"
        );
    }
}
