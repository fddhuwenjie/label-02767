use rocket::{Route, get, State};
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_dashboard]
}

#[get("/dashboard")]
pub async fn admin_dashboard(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    // 统计数据
    let (total_users,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let (total_articles,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let (total_orders,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'paid'")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let total_revenue: f64 = sqlx::query_scalar::<_, rust_decimal::Decimal>("SELECT COALESCE(SUM(amount), 0) FROM orders WHERE status = 'paid'")
        .fetch_one(pool.inner())
        .await
        .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0);

    // 最近订单
    let recent_order_rows: Vec<crate::models::OrderRow> = sqlx::query_as(
        "SELECT * FROM orders ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let recent_orders: Vec<Order> = recent_order_rows.into_iter().map(Order::from).collect();

    // 月度收入统计 - 使用自定义结构体
    #[derive(sqlx::FromRow)]
    struct MonthlyRevenueRow {
        month: Option<String>,
        revenue: rust_decimal::Decimal,
    }
    
    let monthly_rows: Vec<MonthlyRevenueRow> = sqlx::query_as(
        "SELECT DATE_FORMAT(paid_at, '%Y-%m') as month, COALESCE(SUM(amount), 0) as revenue FROM orders WHERE status = 'paid' AND paid_at >= DATE_SUB(NOW(), INTERVAL 6 MONTH) GROUP BY month ORDER BY month"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let monthly_revenue: Vec<(String, f64)> = monthly_rows
        .into_iter()
        .map(|r| (r.month.unwrap_or_default(), r.revenue.to_string().parse::<f64>().unwrap_or(0.0)))
        .collect();

    Template::render("admin/dashboard", context! {
        user: admin.0,
        total_users: total_users,
        total_articles: total_articles,
        total_orders: total_orders,
        total_revenue: total_revenue,
        recent_orders: recent_orders,
        monthly_revenue: monthly_revenue,
    })
}
