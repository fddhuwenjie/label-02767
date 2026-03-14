use sqlx::MySqlPool;
use rust_decimal::Decimal;
use crate::models::{Order, OrderRow};
use crate::models::common::MonthlyRevenue;

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Option<Order> {
    sqlx::query_as::<_, OrderRow>("SELECT * FROM orders WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
        .map(Order::from)
}

pub async fn create_order(
    pool: &MySqlPool,
    id: &str,
    user_id: &str,
    membership_type: &str,
    amount: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO orders (id, user_id, membership_type, amount, status) VALUES (?, ?, ?, ?, 'pending')",
    )
    .bind(id)
    .bind(user_id)
    .bind(membership_type)
    .bind(amount)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_orders(pool: &MySqlPool, page: i64, per_page: i64) -> (Vec<Order>, i64) {
    let total = count_orders(pool).await;
    let offset = (page - 1) * per_page;
    let rows = sqlx::query_as::<_, OrderRow>(
        "SELECT * FROM orders ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    (rows.into_iter().map(Order::from).collect(), total)
}

pub async fn update_status(pool: &MySqlPool, id: &str, status: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE orders SET status = ? WHERE id = ?")
        .bind(status)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn count_orders(pool: &MySqlPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM orders")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn total_revenue(pool: &MySqlPool) -> f64 {
    let result = sqlx::query_scalar::<_, Option<Decimal>>(
        "SELECT SUM(amount) FROM orders WHERE status = 'paid'",
    )
    .fetch_one(pool)
    .await
    .unwrap_or(None);
    result
        .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0)
}

#[derive(sqlx::FromRow)]
struct MonthlyRevenueRow {
    pub month: String,
    pub revenue: Decimal,
}

pub async fn monthly_revenue(pool: &MySqlPool) -> Vec<MonthlyRevenue> {
    let rows = sqlx::query_as::<_, MonthlyRevenueRow>(
        "SELECT DATE_FORMAT(created_at, '%Y-%m') AS month, SUM(amount) AS revenue \
         FROM orders WHERE status = 'paid' \
         GROUP BY DATE_FORMAT(created_at, '%Y-%m') \
         ORDER BY month DESC LIMIT 6",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    rows.into_iter()
        .map(|r| MonthlyRevenue {
            month: r.month,
            revenue: r.revenue.to_string().parse::<f64>().unwrap_or(0.0),
        })
        .collect()
}
