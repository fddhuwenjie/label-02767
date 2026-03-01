use rocket::{Route, get, post, State};
use rocket::serde::json::Json;
use sqlx::MySqlPool;
use chrono::Utc;
use std::collections::BTreeMap;

use crate::models::*;
use crate::middleware::CurrentUser;
use crate::payment::{self, PaymentConfig};

pub fn routes() -> Vec<Route> {
    routes![
        create_order,
        pay_order,
        get_pricing,
        payment_callback_alipay,
        payment_callback_wechat,
        payment_return,
        order_status,
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

#[derive(serde::Serialize)]
pub struct PaymentResponse {
    pub payment_url: Option<String>,
    pub qr_code_url: Option<String>,
    pub order_id: String,
}

#[post("/orders/pay", data = "<form>")]
pub async fn pay_order(
    pool: &State<MySqlPool>,
    user: CurrentUser,
    form: Json<PayOrderRequest>,
) -> Json<ApiResponse<PaymentResponse>> {
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

    let config = PaymentConfig::from_env();

    let membership_name = match order.membership_type.as_str() {
        "monthly" => "月度会员",
        "quarterly" => "季度会员",
        "yearly" => "年度会员",
        "permanent" => "永久会员",
        _ => "会员服务",
    };
    let subject = format!("虚拟资源平台-{}", membership_name);

    match form.payment_method.as_str() {
        "alipay" => {
            if config.alipay_configured() {
                let payment_url = payment::generate_alipay_form(
                    &config,
                    &order.id,
                    order.amount,
                    &subject,
                );

                let _ = sqlx::query(
                    "UPDATE orders SET payment_method = 'alipay' WHERE id = ?"
                )
                .bind(&order.id)
                .execute(pool.inner())
                .await;

                tracing::info!("创建支付宝支付: order={}, amount={}", order.id, order.amount);

                Json(ApiResponse::success(PaymentResponse {
                    payment_url: Some(payment_url),
                    qr_code_url: None,
                    order_id: order.id,
                }, "请完成支付宝支付"))
            } else {
                process_direct_payment(pool, &order, &user, "alipay").await
            }
        }
        "wechat" => {
            if config.wechat_configured() {
                let params = payment::generate_wechat_pay_params(
                    &config,
                    &order.id,
                    order.amount,
                    &subject,
                    "127.0.0.1",
                );
                let xml_body = payment::params_to_xml(&params);

                let client = reqwest::Client::new();
                let resp = client
                    .post("https://api.mch.weixin.qq.com/pay/unifiedorder")
                    .header("Content-Type", "application/xml")
                    .body(xml_body)
                    .send()
                    .await;

                match resp {
                    Ok(r) => {
                        let body = r.text().await.unwrap_or_default();
                        let result_map = payment::parse_xml_to_map(&body);

                        if result_map.get("return_code").map(|s| s.as_str()) == Some("SUCCESS")
                            && result_map.get("result_code").map(|s| s.as_str()) == Some("SUCCESS")
                        {
                            let code_url = result_map.get("code_url").cloned();

                            let _ = sqlx::query(
                                "UPDATE orders SET payment_method = 'wechat' WHERE id = ?"
                            )
                            .bind(&order.id)
                            .execute(pool.inner())
                            .await;

                            tracing::info!("创建微信支付: order={}, amount={}", order.id, order.amount);

                            Json(ApiResponse::success(PaymentResponse {
                                payment_url: None,
                                qr_code_url: code_url,
                                order_id: order.id,
                            }, "请扫码完成微信支付"))
                        } else {
                            let err_msg = result_map.get("return_msg")
                                .cloned()
                                .unwrap_or_else(|| "微信支付下单失败".to_string());
                            tracing::error!("微信支付下单失败: {}", err_msg);
                            process_direct_payment(pool, &order, &user, "wechat").await
                        }
                    }
                    Err(e) => {
                        tracing::error!("微信支付请求失败: {}", e);
                        process_direct_payment(pool, &order, &user, "wechat").await
                    }
                }
            } else {
                process_direct_payment(pool, &order, &user, "wechat").await
            }
        }
        _ => Json(ApiResponse::error("不支持的支付方式")),
    }
}

async fn process_direct_payment(
    pool: &State<MySqlPool>,
    order: &Order,
    user: &CurrentUser,
    method: &str,
) -> Json<ApiResponse<PaymentResponse>> {
    tracing::warn!(
        "支付网关未配置，使用直接支付模式: order={}, method={}",
        order.id, method
    );

    let _ = sqlx::query(
        "UPDATE orders SET status = 'paid', payment_method = ?, paid_at = NOW() WHERE id = ?"
    )
    .bind(method)
    .bind(&order.id)
    .execute(pool.inner())
    .await;

    update_user_membership(pool, &user.0.id, &order.membership_type, &user.0).await;

    tracing::info!("用户 {} 购买会员: {} (直接支付)", user.0.username, order.membership_type);

    Json(ApiResponse::success(PaymentResponse {
        payment_url: None,
        qr_code_url: None,
        order_id: order.id.clone(),
    }, "支付成功"))
}

async fn update_user_membership(
    pool: &State<MySqlPool>,
    user_id: &str,
    membership_type: &str,
    user: &User,
) {
    let days: i64 = match membership_type {
        "monthly" => 30,
        "quarterly" => 90,
        "yearly" => 365,
        _ => 0,
    };

    if membership_type == "permanent" {
        let _ = sqlx::query(
            "UPDATE users SET membership_type = 'permanent', membership_expires_at = NULL WHERE id = ?"
        )
        .bind(user_id)
        .execute(pool.inner())
        .await;
    } else if days > 0 {
        let current_expires = user.membership_expires_at.unwrap_or(Utc::now());
        let base_time = if current_expires > Utc::now() { current_expires } else { Utc::now() };
        let new_expires = base_time + chrono::Duration::days(days);

        let _ = sqlx::query(
            "UPDATE users SET membership_type = ?, membership_expires_at = ? WHERE id = ?"
        )
        .bind(membership_type)
        .bind(new_expires)
        .bind(user_id)
        .execute(pool.inner())
        .await;
    }
}

/// 支付宝异步回调通知
#[post("/payment/callback/alipay", data = "<body>")]
pub async fn payment_callback_alipay(
    pool: &State<MySqlPool>,
    body: String,
) -> &'static str {
    tracing::info!("收到支付宝回调通知");

    let params: BTreeMap<String, String> = body
        .split('&')
        .filter_map(|pair| {
            let mut parts = pair.splitn(2, '=');
            let key = parts.next()?.to_string();
            let value = parts.next().unwrap_or("").to_string();
            Some((key, value))
        })
        .collect();

    let config = PaymentConfig::from_env();

    if !config.alipay_public_key.is_empty() {
        if !payment::verify_alipay_callback(&params, &config.alipay_public_key) {
            tracing::warn!("支付宝回调签名验证失败");
            return "fail";
        }
    }

    let trade_status = params.get("trade_status").map(|s| s.as_str()).unwrap_or("");
    let order_id = match params.get("out_trade_no") {
        Some(id) => id,
        None => {
            tracing::warn!("支付宝回调缺少 out_trade_no");
            return "fail";
        }
    };

    if trade_status == "TRADE_SUCCESS" || trade_status == "TRADE_FINISHED" {
        if let Err(e) = complete_order_payment(pool, order_id, "alipay").await {
            tracing::error!("处理支付宝回调订单失败: {}", e);
            return "fail";
        }
        tracing::info!("支付宝回调处理成功: order={}", order_id);
    }

    "success"
}

/// 微信支付异步回调通知
#[post("/payment/callback/wechat", data = "<body>")]
pub async fn payment_callback_wechat(
    pool: &State<MySqlPool>,
    body: String,
) -> String {
    tracing::info!("收到微信支付回调通知");

    let params = payment::parse_xml_to_map(&body);
    let config = PaymentConfig::from_env();

    if !config.wechat_api_key.is_empty() {
        if !payment::verify_wechat_callback(&params, &config.wechat_api_key) {
            tracing::warn!("微信支付回调签名验证失败");
            return "<xml><return_code><![CDATA[FAIL]]></return_code><return_msg><![CDATA[签名失败]]></return_msg></xml>".to_string();
        }
    }

    let result_code = params.get("result_code").map(|s| s.as_str()).unwrap_or("");
    let order_id = match params.get("out_trade_no") {
        Some(id) => id,
        None => {
            tracing::warn!("微信支付回调缺少 out_trade_no");
            return "<xml><return_code><![CDATA[FAIL]]></return_code><return_msg><![CDATA[缺少参数]]></return_msg></xml>".to_string();
        }
    };

    if result_code == "SUCCESS" {
        if let Err(e) = complete_order_payment(pool, order_id, "wechat").await {
            tracing::error!("处理微信支付回调订单失败: {}", e);
            return "<xml><return_code><![CDATA[FAIL]]></return_code><return_msg><![CDATA[处理失败]]></return_msg></xml>".to_string();
        }
        tracing::info!("微信支付回调处理成功: order={}", order_id);
    }

    "<xml><return_code><![CDATA[SUCCESS]]></return_code><return_msg><![CDATA[OK]]></return_msg></xml>".to_string()
}

async fn complete_order_payment(
    pool: &State<MySqlPool>,
    order_id: &str,
    payment_method: &str,
) -> Result<(), String> {
    let order_row: Option<OrderRow> = sqlx::query_as(
        "SELECT * FROM orders WHERE id = ? AND status = 'pending'"
    )
    .bind(order_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let order = match order_row {
        Some(o) => Order::from(o),
        None => return Ok(()),
    };

    let _ = sqlx::query(
        "UPDATE orders SET status = 'paid', payment_method = ?, paid_at = NOW() WHERE id = ? AND status = 'pending'"
    )
    .bind(payment_method)
    .bind(order_id)
    .execute(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE id = ?"
    )
    .bind(&order.user_id)
    .fetch_optional(pool.inner())
    .await
    .map_err(|e| e.to_string())?;

    if let Some(user) = user {
        let days: i64 = match order.membership_type.as_str() {
            "monthly" => 30,
            "quarterly" => 90,
            "yearly" => 365,
            _ => 0,
        };

        if order.membership_type == "permanent" {
            let _ = sqlx::query(
                "UPDATE users SET membership_type = 'permanent', membership_expires_at = NULL WHERE id = ?"
            )
            .bind(&order.user_id)
            .execute(pool.inner())
            .await;
        } else if days > 0 {
            let current_expires = user.membership_expires_at.unwrap_or(Utc::now());
            let base_time = if current_expires > Utc::now() { current_expires } else { Utc::now() };
            let new_expires = base_time + chrono::Duration::days(days);

            let _ = sqlx::query(
                "UPDATE users SET membership_type = ?, membership_expires_at = ? WHERE id = ?"
            )
            .bind(&order.membership_type)
            .bind(new_expires)
            .bind(&order.user_id)
            .execute(pool.inner())
            .await;
        }

        tracing::info!(
            "用户 {} 会员更新: {} (回调确认)",
            user.username, order.membership_type
        );
    }

    Ok(())
}

/// 支付宝同步返回页面（用户支付后跳转回来）
#[get("/payment/return?<out_trade_no>")]
pub async fn payment_return(
    pool: &State<MySqlPool>,
    out_trade_no: Option<String>,
) -> rocket::response::Redirect {
    if let Some(order_id) = out_trade_no {
        tracing::info!("支付宝同步返回: order={}", order_id);

        let order: Option<(String,)> = sqlx::query_as(
            "SELECT status FROM orders WHERE id = ?"
        )
        .bind(&order_id)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

        if let Some((status,)) = order {
            if status == "paid" {
                return rocket::response::Redirect::to("/profile");
            }
        }
    }

    rocket::response::Redirect::to("/profile")
}

/// 前端轮询订单支付状态
#[get("/orders/<order_id>/status")]
pub async fn order_status(
    pool: &State<MySqlPool>,
    user: CurrentUser,
    order_id: String,
) -> Json<ApiResponse<OrderStatusResponse>> {
    let order: Option<(String,)> = sqlx::query_as(
        "SELECT status FROM orders WHERE id = ? AND user_id = ?"
    )
    .bind(&order_id)
    .bind(&user.0.id)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    match order {
        Some((status,)) => Json(ApiResponse::success(
            OrderStatusResponse { status: status.clone(), paid: status == "paid" },
            "查询成功",
        )),
        None => Json(ApiResponse::error("订单不存在")),
    }
}

#[derive(serde::Serialize)]
pub struct OrderStatusResponse {
    pub status: String,
    pub paid: bool,
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
