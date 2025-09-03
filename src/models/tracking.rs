use chrono::{DateTime, Utc};
use clickhouse::Row;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct AuthTracking {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAuthTrackingRequest {
    pub user_id: Option<Uuid>,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Row, Serialize, Deserialize)]
pub struct BlogTracking {
    pub id: Uuid,
    pub blog_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub session_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBlogTrackingRequest {
    pub blog_id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub referrer: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingStats {
    pub total_views: u64,
    pub unique_visitors: u64,
    pub top_pages: Vec<PageStats>,
    pub recent_activity: Vec<ActivitySummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageStats {
    pub path: String,
    pub views: u64,
    pub unique_visitors: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivitySummary {
    pub action: String,
    pub count: u64,
    pub last_activity: DateTime<Utc>,
}
