use serde::{Deserialize, Serialize};

use super::order::Order;

/// 通用 API 响应
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub message: String,
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
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

/// 后台仪表盘统计
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
