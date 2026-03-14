use rocket::{Route, get, post, delete, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_categories, admin_category_create, admin_category_delete]
}

#[derive(serde::Deserialize)]
pub struct CategoryRequest {
    pub name: String,
    pub sort_order: Option<i32>,
}

#[get("/categories")]
pub async fn admin_categories(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    let categories: Vec<Category> = sqlx::query_as("SELECT * FROM categories ORDER BY sort_order")
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

    Template::render("admin/categories", context! {
        user: admin.0,
        categories: categories,
    })
}

#[post("/categories", data = "<form>")]
pub async fn admin_category_create(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    form: Json<CategoryRequest>,
) -> Json<ApiResponse<String>> {
    let id = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query("INSERT INTO categories (id, name, sort_order) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&form.name)
        .bind(form.sort_order.unwrap_or(0))
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success(id, "创建成功")),
        Err(_) => Json(ApiResponse::error("创建失败")),
    }
}

#[delete("/categories/<id>")]
pub async fn admin_category_delete(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "删除成功")),
        Err(_) => Json(ApiResponse::error("删除失败")),
    }
}
