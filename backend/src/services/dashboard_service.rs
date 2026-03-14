use sqlx::MySqlPool;
use crate::models::{DashboardStats, Order, OrderRow};
use crate::models::common::MonthlyRevenue;
use crate::services::{user_service, article_service, order_service};

pub async fn get_stats(pool: &MySqlPool) -> DashboardStats {
    let total_users = user_service::count_users(pool).await;
    let total_articles = article_service::count_articles(pool).await;
    let total_orders = order_service::count_orders(pool).await;
    let total_revenue = order_service::total_revenue(pool).await;

    let recent_rows = sqlx::query_as::<_, OrderRow>(
        "SELECT * FROM orders ORDER BY created_at DESC LIMIT 10",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    let recent_orders: Vec<Order> = recent_rows.into_iter().map(Order::from).collect();

    let monthly_revenue = order_service::monthly_revenue(pool).await;

    DashboardStats {
        total_users,
        total_articles,
        total_orders,
        total_revenue,
        recent_orders,
        monthly_revenue,
    }
}
