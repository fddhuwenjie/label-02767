use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(0.0)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
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
    pub name: String,
    pub price: f64,
    pub duration_days: i32,
    pub description: Option<String>,
    pub features: Vec<String>,
    pub is_active: bool,
}

impl From<PricingRow> for Pricing {
    fn from(row: PricingRow) -> Self {
        let (name, duration_days, features) = match row.membership_type.as_str() {
            "monthly" => ("月度会员".to_string(), 30, vec![
                "全站资源无限下载".to_string(),
                "30天有效期".to_string(),
                "优先客服支持".to_string(),
            ]),
            "quarterly" => ("季度会员".to_string(), 90, vec![
                "全站资源无限下载".to_string(),
                "90天有效期".to_string(),
                "优先客服支持".to_string(),
                "比月度节省10%".to_string(),
            ]),
            "yearly" => ("年度会员".to_string(), 365, vec![
                "全站资源无限下载".to_string(),
                "365天有效期".to_string(),
                "优先客服支持".to_string(),
                "比月度节省43%".to_string(),
                "专属会员标识".to_string(),
            ]),
            "permanent" => ("永久会员".to_string(), -1, vec![
                "全站资源无限下载".to_string(),
                "终身有效，永不过期".to_string(),
                "优先客服支持".to_string(),
                "专属会员标识".to_string(),
                "一次购买永久使用".to_string(),
            ]),
            _ => (row.membership_type.clone(), 0, vec![]),
        };

        Self {
            id: row.id,
            membership_type: row.membership_type,
            name,
            price: decimal_to_f64(row.price),
            duration_days,
            description: row.description,
            features,
            is_active: row.is_active,
        }
    }
}
