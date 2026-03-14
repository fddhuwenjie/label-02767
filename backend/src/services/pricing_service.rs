use sqlx::MySqlPool;
use crate::models::{Pricing, PricingRow};

pub async fn list_active(pool: &MySqlPool) -> Vec<Pricing> {
    let rows = sqlx::query_as::<_, PricingRow>(
        "SELECT * FROM pricing WHERE is_active = true ORDER BY sort_order ASC",
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    rows.into_iter().map(Pricing::from).collect()
}

pub async fn list_all(pool: &MySqlPool) -> Vec<Pricing> {
    let rows = sqlx::query_as::<_, PricingRow>("SELECT * FROM pricing ORDER BY sort_order ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    rows.into_iter().map(Pricing::from).collect()
}

pub async fn find_by_type(pool: &MySqlPool, membership_type: &str) -> Option<Pricing> {
    sqlx::query_as::<_, PricingRow>(
        "SELECT * FROM pricing WHERE membership_type = ? AND is_active = true",
    )
    .bind(membership_type)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()
    .map(Pricing::from)
}

pub async fn update(
    pool: &MySqlPool,
    id: &str,
    price: f64,
    duration_days: i32,
    description: Option<&str>,
    features: Option<&str>,
    is_active: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE pricing SET price = ?, duration_days = ?, description = ?, features = ?, is_active = ? WHERE id = ?",
    )
    .bind(price)
    .bind(duration_days)
    .bind(description)
    .bind(features)
    .bind(is_active)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}
