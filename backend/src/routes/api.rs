use rocket::{Route, get, post, State};
use rocket::serde::json::Json;
use sqlx::MySqlPool;
use chrono::Utc;

use crate::models::*;
use crate::middleware::CurrentUser;

pub fn routes() -> Vec<Route> {
    routes![
        create_order,
        pay_order,
        get_pricing,
    ]
}

#[derive(serde::Deserialize)]
pub struct CreateOrderRequest {
    pub membership_type: String,
}

#[post("/orders", data = "<form>")]
pub async fn create_order(
    pool: &State<MySqlPool>,
    user: CurrentUser,
    form: Json<CreateOrderRequest>,
) -> Json<ApiResponse<Order>> {
    // 获取定价
    let pricing_row: Option<PricingRow> = sqlx::query_as(
        "SELECT * FROM pricing WHERE membership_type = ? AND is_active = TRUE"
    )
    .bind(&form.membership_type)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    let pricing = match pricing_row {
        Some(p) => Pricing::from(p),
        None => return Json(ApiResponse::error("无效的会员类型")),
    };

    let order_id = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query(
        "INSERT INTO orders (id, user_id, membership_type, amount, status) VALUES (?, ?, ?, ?, 'pending')"
    )
    .bind(&order_id)
    .bind(&user.0.id)
    .bind(&form.membership_type)
    .bind(pricing.price)
    .execute(pool.inner())
    .await;

    match result {
        Ok(_) => {
            let order_row: OrderRow = sqlx::query_as("SELECT * FROM orders WHERE id = ?")
                .bind(&order_id)
                .fetch_one(pool.inner())
                .await
                .unwrap();
            Json(ApiResponse::success(Order::from(order_row), "订单创建成功"))
        }
        Err(_) => Json(ApiResponse::error("创建订单失败")),
    }
}

#[derive(serde::Deserialize)]
pub struct PayOrderRequest {
    pub order_id: String,
    pub payment_method: String,
}

#[post("/orders/pay", data = "<form>")]
pub async fn pay_order(
    pool: &State<MySqlPool>,
    user: CurrentUser,
    form: Json<PayOrderRequest>,
) -> Json<ApiResponse<()>> {
    // 查找订单
    let order_row: Option<OrderRow> = sqlx::query_as(
        "SELECT * FROM orders WHERE id = ? AND user_id = ? AND status = 'pending'"
    )
    .bind(&form.order_id)
    .bind(&user.0.id)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    let order = match order_row {
        Some(o) => Order::from(o),
        None => return Json(ApiResponse::error("订单不存在或已支付")),
    };

    // 模拟支付成功（实际项目中需要对接支付网关）
    let _ = sqlx::query(
        "UPDATE orders SET status = 'paid', payment_method = ?, paid_at = NOW() WHERE id = ?"
    )
    .bind(&form.payment_method)
    .bind(&order.id)
    .execute(pool.inner())
    .await;

    // 更新用户会员状态
    let days = match order.membership_type.as_str() {
        "monthly" => 30,
        "quarterly" => 90,
        "yearly" => 365,
        "permanent" => 0,
        _ => 0,
    };

    if order.membership_type == "permanent" {
        let _ = sqlx::query(
            "UPDATE users SET membership_type = 'permanent', membership_expires_at = NULL WHERE id = ?"
        )
        .bind(&user.0.id)
        .execute(pool.inner())
        .await;
    } else if days > 0 {
        // 如果已有会员，则延长时间
        let current_expires = user.0.membership_expires_at.unwrap_or(Utc::now());
        let base_time = if current_expires > Utc::now() { current_expires } else { Utc::now() };
        let new_expires = base_time + chrono::Duration::days(days);

        let _ = sqlx::query(
            "UPDATE users SET membership_type = ?, membership_expires_at = ? WHERE id = ?"
        )
        .bind(&order.membership_type)
        .bind(new_expires)
        .bind(&user.0.id)
        .execute(pool.inner())
        .await;
    }

    tracing::info!("用户 {} 购买会员: {}", user.0.username, order.membership_type);
    Json(ApiResponse::success((), "支付成功"))
}

#[get("/pricing")]
pub async fn get_pricing(
    pool: &State<MySqlPool>,
) -> Json<ApiResponse<Vec<Pricing>>> {
    let pricing_rows: Vec<PricingRow> = sqlx::query_as(
        "SELECT * FROM pricing WHERE is_active = TRUE ORDER BY price"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let pricing: Vec<Pricing> = pricing_rows.into_iter().map(Pricing::from).collect();

    Json(ApiResponse::success(pricing, "获取成功"))
}
