use rocket::{Route, get, put, delete, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_tags, admin_tag_update, admin_tag_delete]
}

#[derive(serde::Deserialize)]
pub struct TagUpdateRequest {
    pub name: String,
}

#[get("/tags?<page>")]
pub async fn admin_tags(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    page: Option<i32>,
) -> Template {
    let page = page.unwrap_or(1).max(1);
    let per_page = 20;
    let offset = (page - 1) * per_page;

    #[derive(sqlx::FromRow, serde::Serialize)]
    struct TagWithCount {
        id: String,
        name: String,
        created_at: chrono::DateTime<chrono::Utc>,
        article_count: i64,
    }

    let tags: Vec<TagWithCount> = sqlx::query_as(
        "SELECT t.id, t.name, t.created_at, COUNT(at.article_id) as article_count FROM tags t LEFT JOIN article_tags at ON t.id = at.tag_id GROUP BY t.id, t.name, t.created_at ORDER BY article_count DESC, t.created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tags")
        .fetch_one(pool.inner()).await.unwrap_or((0,));

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("admin/tags", context! {
        user: admin.0,
        tags: tags,
        current_page: page,
        total_pages: total_pages,
    })
}

#[put("/tags/<id>", data = "<form>")]
pub async fn admin_tag_update(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
    form: Json<TagUpdateRequest>,
) -> Json<ApiResponse<()>> {
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT id FROM tags WHERE name = ? AND id != ?"
    )
    .bind(&form.name)
    .bind(&id)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    if existing.is_some() {
        return Json(ApiResponse::error("标签名称已存在"));
    }

    let result = sqlx::query("UPDATE tags SET name = ? WHERE id = ?")
        .bind(&form.name)
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "更新成功")),
        Err(_) => Json(ApiResponse::error("更新失败")),
    }
}

#[delete("/tags/<id>")]
pub async fn admin_tag_delete(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
) -> Json<ApiResponse<()>> {
    let _ = sqlx::query("DELETE FROM article_tags WHERE tag_id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    let result = sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "删除成功")),
        Err(_) => Json(ApiResponse::error("删除失败")),
    }
}
