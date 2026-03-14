use rocket::{Route, get, State};
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_orders]
}

#[get("/orders?<page>")]
pub async fn admin_orders(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    page: Option<i32>,
) -> Template {
    let page = page.unwrap_or(1).max(1);
    let per_page = 20;
    let offset = (page - 1) * per_page;

    let order_rows: Vec<crate::models::OrderRow> = sqlx::query_as(
        "SELECT * FROM orders ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let orders: Vec<Order> = order_rows.into_iter().map(Order::from).collect();

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders")
        .fetch_one(pool.inner()).await.unwrap_or((0,));

    // 统计各状态订单数量
    let (paid_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'paid'")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    let (pending_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'pending'")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    let (cancelled_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'cancelled'")
        .fetch_one(pool.inner()).await.unwrap_or((0,));

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("admin/orders", context! {
        user: admin.0,
        orders: orders,
        current_page: page,
        total_pages: total_pages,
        paid_count: paid_count,
        pending_count: pending_count,
        cancelled_count: cancelled_count,
    })
}
