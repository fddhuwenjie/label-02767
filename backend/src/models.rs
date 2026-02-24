use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

// 自定义 Decimal 到 f64 的转换
fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub is_admin: bool,
    pub membership_type: String,
    pub membership_expires_at: Option<DateTime<Utc>>,
    pub points: i32,
    pub is_frozen: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn has_valid_membership(&self) -> bool {
        if self.is_admin || self.membership_type == "permanent" {
            return true;
        }
        if self.membership_type == "none" {
            return false;
        }
        if let Some(expires) = self.membership_expires_at {
            return expires > Utc::now();
        }
        false
    }
}

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
    pub content_html: String,
    pub cover_image: Option<String>,
    pub category_name: Option<String>,
    pub platform_name: Option<String>,
    pub disk_link: Option<String>,
    pub disk_password: Option<String>,
    pub is_link_valid: bool,
    pub view_count: i32,
    pub is_featured: bool,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Category {
    pub id: String,
    pub name: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Tag {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct DiskPlatform {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// 用于数据库查询的 Order 结构体
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct OrderRow {
    pub id: String,
    pub user_id: String,
    pub membership_type: String,
    pub amount: Decimal,
    pub status: String,
    pub payment_method: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub user_id: String,
    pub membership_type: String,
    pub amount: f64,
    pub status: String,
    pub payment_method: Option<String>,
    pub paid_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

impl From<OrderRow> for Order {
    fn from(row: OrderRow) -> Self {
        Order {
            id: row.id,
            user_id: row.user_id,
            membership_type: row.membership_type,
            amount: decimal_to_f64(row.amount),
            status: row.status,
            payment_method: row.payment_method,
            paid_at: row.paid_at,
            created_at: row.created_at,
        }
    }
}

// 用于数据库查询的 Pricing 结构体
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PricingRow {
    pub id: String,
    pub membership_type: String,
    pub price: Decimal,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pricing {
    pub id: String,
    pub membership_type: String,
    pub price: f64,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PricingRow> for Pricing {
    fn from(row: PricingRow) -> Self {
        Pricing {
            id: row.id,
            membership_type: row.membership_type,
            price: decimal_to_f64(row.price),
            description: row.description,
            is_active: row.is_active,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// 请求/响应结构体
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T, message: &str) -> Self {
        Self {
            success: true,
            message: message.to_string(),
            data: Some(data),
        }
    }

    pub fn error(message: &str) -> Self {
        Self {
            success: false,
            message: message.to_string(),
            data: None,
        }
    }
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
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct PricingUpdateRequest {
    pub price: f64,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub membership_type: Option<String>,
    pub points: Option<i32>,
    pub is_frozen: Option<bool>,
    pub is_admin: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub total_users: i64,
    pub total_articles: i64,
    pub total_orders: i64,
    pub total_revenue: f64,
    pub recent_orders: Vec<Order>,
    pub monthly_revenue: Vec<MonthlyRevenue>,
}

#[derive(Debug, Serialize)]
pub struct MonthlyRevenue {
    pub month: String,
    pub revenue: f64,
}
