use rocket::{Route, get, put, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_pricing, admin_pricing_update]
}

#[derive(serde::Deserialize)]
pub struct PricingUpdateRequest {
    pub price: f64,
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[get("/pricing")]
pub async fn admin_pricing(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    let pricing_rows: Vec<crate::models::PricingRow> = sqlx::query_as("SELECT * FROM pricing ORDER BY price")
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();
    
    let pricing: Vec<Pricing> = pricing_rows.into_iter().map(Pricing::from).collect();

    Template::render("admin/pricing", context! {
        user: admin.0,
        pricing: pricing,
    })
}

#[put("/pricing/<id>", data = "<form>")]
pub async fn admin_pricing_update(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
    form: Json<PricingUpdateRequest>,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query(
        "UPDATE pricing SET price = ?, description = ?, is_active = ? WHERE id = ?"
    )
    .bind(form.price)
    .bind(&form.description)
    .bind(form.is_active.unwrap_or(true))
    .bind(&id)
    .execute(pool.inner())
    .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "更新成功")),
        Err(_) => Json(ApiResponse::error("更新失败")),
    }
}
