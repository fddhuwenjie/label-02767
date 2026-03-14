use rocket::{Route, get, post, put, delete, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![
        admin_articles,
        admin_article_create_page,
        admin_article_create,
        admin_article_edit_page,
        admin_article_update,
        admin_article_delete,
    ]
}

#[get("/articles?<page>")]
pub async fn admin_articles(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    page: Option<i32>,
) -> Template {
    let page = page.unwrap_or(1).max(1);
    let per_page = 20;
    let offset = (page - 1) * per_page;

    let articles: Vec<Article> = sqlx::query_as(
        "SELECT * FROM articles ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles")
        .fetch_one(pool.inner()).await.unwrap_or((0,));

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("admin/articles", context! {
        user: admin.0,
        articles: articles,
        current_page: page,
        total_pages: total_pages,
    })
}

#[get("/articles/create")]
pub async fn admin_article_create_page(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    let categories: Vec<Category> = sqlx::query_as("SELECT * FROM categories ORDER BY sort_order")
        .fetch_all(pool.inner()).await.unwrap_or_default();
    
    let platforms: Vec<DiskPlatform> = sqlx::query_as("SELECT * FROM disk_platforms WHERE is_active = TRUE")
        .fetch_all(pool.inner()).await.unwrap_or_default();

    Template::render("admin/article_edit", context! {
        user: admin.0,
        categories: categories,
        platforms: platforms,
        is_new: true,
    })
}

#[post("/articles", data = "<form>")]
pub async fn admin_article_create(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    form: Json<ArticleCreateRequest>,
) -> Json<ApiResponse<String>> {
    let id = uuid::Uuid::new_v4().to_string();

    let result = sqlx::query(
        "INSERT INTO articles (id, title, content, cover_image, category_id, disk_platform_id, disk_link, disk_password, is_visible, is_featured, is_link_valid) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&form.title)
    .bind(&form.content)
    .bind(&form.cover_image)
    .bind(&form.category_id)
    .bind(&form.disk_platform_id)
    .bind(&form.disk_link)
    .bind(&form.disk_password)
    .bind(form.is_visible.unwrap_or(true))
    .bind(form.is_featured.unwrap_or(false))
    .bind(form.is_link_valid.unwrap_or(true))
    .execute(pool.inner())
    .await;

    match result {
        Ok(_) => {
            // 处理标签
            if let Some(tags) = &form.tags {
                for tag_name in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    // 查找或创建标签
                    let tag_id: Option<String> = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
                        .bind(tag_name)
                        .fetch_optional(pool.inner())
                        .await
                        .ok()
                        .flatten();

                    let tag_id = match tag_id {
                        Some(id) => id,
                        None => {
                            let new_id = uuid::Uuid::new_v4().to_string();
                            let _ = sqlx::query("INSERT INTO tags (id, name) VALUES (?, ?)")
                                .bind(&new_id)
                                .bind(tag_name)
                                .execute(pool.inner())
                                .await;
                            new_id
                        }
                    };

                    let _ = sqlx::query("INSERT IGNORE INTO article_tags (article_id, tag_id) VALUES (?, ?)")
                        .bind(&id)
                        .bind(&tag_id)
                        .execute(pool.inner())
                        .await;
                }
            }
            Json(ApiResponse::success(id, "文章创建成功"))
        }
        Err(e) => {
            tracing::error!("创建文章失败: {}", e);
            Json(ApiResponse::error("创建失败"))
        }
    }
}

#[get("/articles/<id>/edit")]
pub async fn admin_article_edit_page(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    id: String,
) -> Option<Template> {
    let article: Option<Article> = sqlx::query_as("SELECT * FROM articles WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .ok()?;

    let article = article?;

    let categories: Vec<Category> = sqlx::query_as("SELECT * FROM categories ORDER BY sort_order")
        .fetch_all(pool.inner()).await.unwrap_or_default();
    
    let platforms: Vec<DiskPlatform> = sqlx::query_as("SELECT * FROM disk_platforms WHERE is_active = TRUE")
        .fetch_all(pool.inner()).await.unwrap_or_default();

    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT t.name FROM tags t JOIN article_tags at ON t.id = at.tag_id WHERE at.article_id = ?"
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    Some(Template::render("admin/article_edit", context! {
        user: admin.0,
        article: article,
        categories: categories,
        platforms: platforms,
        tags: tags,
        is_new: false,
    }))
}

#[put("/articles/<id>", data = "<form>")]
pub async fn admin_article_update(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
    form: Json<ArticleCreateRequest>,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query(
        "UPDATE articles SET title = ?, content = ?, cover_image = ?, category_id = ?, disk_platform_id = ?, disk_link = ?, disk_password = ?, is_visible = ?, is_featured = ?, is_link_valid = ?, updated_at = NOW() WHERE id = ?"
    )
    .bind(&form.title)
    .bind(&form.content)
    .bind(&form.cover_image)
    .bind(&form.category_id)
    .bind(&form.disk_platform_id)
    .bind(&form.disk_link)
    .bind(&form.disk_password)
    .bind(form.is_visible.unwrap_or(true))
    .bind(form.is_featured.unwrap_or(false))
    .bind(form.is_link_valid.unwrap_or(true))
    .bind(&id)
    .execute(pool.inner())
    .await;

    match result {
        Ok(_) => {
            // 更新标签
            let _ = sqlx::query("DELETE FROM article_tags WHERE article_id = ?")
                .bind(&id)
                .execute(pool.inner())
                .await;

            if let Some(tags) = &form.tags {
                for tag_name in tags.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                    let tag_id: Option<String> = sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
                        .bind(tag_name)
                        .fetch_optional(pool.inner())
                        .await
                        .ok()
                        .flatten();

                    let tag_id = match tag_id {
                        Some(tid) => tid,
                        None => {
                            let new_id = uuid::Uuid::new_v4().to_string();
                            let _ = sqlx::query("INSERT INTO tags (id, name) VALUES (?, ?)")
                                .bind(&new_id)
                                .bind(tag_name)
                                .execute(pool.inner())
                                .await;
                            new_id
                        }
                    };

                    let _ = sqlx::query("INSERT IGNORE INTO article_tags (article_id, tag_id) VALUES (?, ?)")
                        .bind(&id)
                        .bind(&tag_id)
                        .execute(pool.inner())
                        .await;
                }
            }
            Json(ApiResponse::success((), "更新成功"))
        }
        Err(_) => Json(ApiResponse::error("更新失败")),
    }
}

#[delete("/articles/<id>")]
pub async fn admin_article_delete(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
) -> Json<ApiResponse<()>> {
    let result = sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(_) => Json(ApiResponse::success((), "删除成功")),
        Err(_) => Json(ApiResponse::error("删除失败")),
    }
}
