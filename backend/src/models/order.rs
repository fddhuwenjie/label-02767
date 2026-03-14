use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
        Self {
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
