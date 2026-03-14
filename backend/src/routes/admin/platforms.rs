use rocket::{Route, get, post, put, delete, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_platforms, admin_platform_create, admin_platform_update, admin_platform_delete]
}

#[derive(serde::Deserialize)]
pub struct PlatformRequest {
    pub name: String,
    pub icon: Option<String>,
}

#[get("/platforms")]
pub async fn admin_platforms(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    let platforms: Vec<DiskPlatform> = sqlx::query_as("SELECT * FROM disk_platforms ORDER BY created_at")
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

    Template::render("admin/platforms", context! {
        user: admin.0,
        platforms: platforms,
    })
}

#[post("/platforms", data = "<form>")]
pub async fn admin_platform_create(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    form: Json<PlatformRequest>,
) -> Json<ApiResponse<String>> {
    let id = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query("INSERT INTO disk_platforms (id, name, icon) VALUES (?, ?, ?)")
        .bind(&id)
        .bind(&form.name)
        .bind(&form.icon)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success(id, "创建成功")),
        Err(_) => Json(ApiResponse::error("创建失败")),
    }
}

#[put("/platforms/<id>", data = "<form>")]
pub async fn admin_platform_update(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
    form: Json<PlatformRequest>,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query("UPDATE disk_platforms SET name = ?, icon = ? WHERE id = ?")
        .bind(&form.name)
        .bind(&form.icon)
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "更新成功")),
        Err(_) => Json(ApiResponse::error("更新失败")),
    }
}

#[delete("/platforms/<id>")]
pub async fn admin_platform_delete(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query("DELETE FROM disk_platforms WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "删除成功")),
        Err(_) => Json(ApiResponse::error("删除失败")),
    }
}
