use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Article {
    pub id: String,
    pub title: String,
    pub content: String,
    pub cover_image: Option<String>,
    pub category_id: Option<String>,
    pub disk_platform_id: Option<String>,
    pub disk_link: Option<String>,
    pub disk_password: Option<String>,
    pub is_link_valid: bool,
    pub view_count: i32,
    pub is_visible: bool,
    pub is_featured: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArticleWithDetails {
    pub id: String,
    pub title: String,
    pub content: String,
    pub cover_image: Option<String>,
    pub category_name: Option<String>,
    pub platform_name: Option<String>,
    pub disk_link: Option<String>,
    pub disk_password: Option<String>,
    pub is_link_valid: bool,
    pub view_count: i32,
    pub is_visible: bool,
    pub is_featured: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ArticleCreateRequest {
    pub title: String,
    pub content: String,
    pub cover_image: Option<String>,
    pub category_id: Option<String>,
    pub disk_platform_id: Option<String>,
    pub disk_link: Option<String>,
    pub disk_password: Option<String>,
    pub is_visible: Option<bool>,
    pub is_featured: Option<bool>,
    pub is_link_valid: Option<bool>,
    pub tags: Option<String>,
}
