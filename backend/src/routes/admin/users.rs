use rocket::{Route, get, post, delete, put, State};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_users, admin_user_update, admin_user_create, admin_user_delete]
}

/// 后台创建用户请求
#[derive(serde::Deserialize)]
pub struct AdminUserCreateRequest {
    pub username: String,
    pub email: String,
    pub password: String,
    pub membership_type: Option<String>,
    pub is_admin: Option<bool>,
    pub points: Option<i32>,
}

#[get("/users?<page>")]
pub async fn admin_users(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    page: Option<i32>,
) -> Template {
    let page = page.unwrap_or(1).max(1);
    let per_page = 20;
    let offset = (page - 1) * per_page;

    let users: Vec<User> = sqlx::query_as(
        "SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?"
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool.inner()).await.unwrap_or((0,));

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("admin/users", context! {
        user: admin.0,
        users: users,
        current_page: page,
        total_pages: total_pages,
    })
}

#[put("/users/<id>", data = "<form>")]
pub async fn admin_user_update(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    id: String,
    form: Json<UserUpdateRequest>,
) -> Json<ApiResponse<()>> {
    // 先获取当前用户信息
    let current_user: Option<User> = sqlx::query_as("SELECT * FROM users WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

    if current_user.is_none() {
        return Json(ApiResponse::error("用户不存在"));
    }

    let membership_type = form.membership_type.clone().unwrap_or_else(|| current_user.as_ref().unwrap().membership_type.clone());
    let points = form.points.unwrap_or(current_user.as_ref().unwrap().points);
    let is_frozen = form.is_frozen.unwrap_or(current_user.as_ref().unwrap().is_frozen);
    let is_admin = form.is_admin.unwrap_or(current_user.as_ref().unwrap().is_admin);

    // 计算会员过期时间
    let membership_expires = match membership_type.as_str() {
        "monthly" => Some(30),
        "quarterly" => Some(90),
        "yearly" => Some(365),
        _ => None,
    };

    let result = if membership_type == "permanent" || membership_type == "none" {
        sqlx::query(
            "UPDATE users SET membership_type = ?, membership_expires_at = NULL, points = ?, is_frozen = ?, is_admin = ? WHERE id = ?"
        )
        .bind(&membership_type)
        .bind(points)
        .bind(is_frozen)
        .bind(is_admin)
        .bind(&id)
        .execute(pool.inner())
        .await
    } else if let Some(days) = membership_expires {
        sqlx::query(
            "UPDATE users SET membership_type = ?, membership_expires_at = DATE_ADD(NOW(), INTERVAL ? DAY), points = ?, is_frozen = ?, is_admin = ? WHERE id = ?"
        )
        .bind(&membership_type)
        .bind(days)
        .bind(points)
        .bind(is_frozen)
        .bind(is_admin)
        .bind(&id)
        .execute(pool.inner())
        .await
    } else {
        sqlx::query(
            "UPDATE users SET membership_type = ?, points = ?, is_frozen = ?, is_admin = ? WHERE id = ?"
        )
        .bind(&membership_type)
        .bind(points)
        .bind(is_frozen)
        .bind(is_admin)
        .bind(&id)
        .execute(pool.inner())
        .await
    };

    match result {
        Ok(_) => Json(ApiResponse::success((), "更新成功")),
        Err(e) => {
            tracing::error!("更新用户失败: {}", e);
            Json(ApiResponse::error("更新失败"))
        }
    }
}

#[post("/users", data = "<form>")]
pub async fn admin_user_create(
    pool: &State<MySqlPool>,
    _admin: AdminUser,
    form: Json<AdminUserCreateRequest>,
) -> Json<ApiResponse<String>> {
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).unwrap();
    let membership_type = form.membership_type.clone().unwrap_or_else(|| "none".to_string());
    let is_admin = form.is_admin.unwrap_or(false);
    let points = form.points.unwrap_or(0);

    // 计算会员过期时间
    let result = match membership_type.as_str() {
        "monthly" => {
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, membership_type, membership_expires_at, is_admin, points) VALUES (?, ?, ?, ?, ?, DATE_ADD(NOW(), INTERVAL 30 DAY), ?, ?)"
            )
            .bind(&id)
            .bind(&form.username)
            .bind(&form.email)
            .bind(&password_hash)
            .bind(&membership_type)
            .bind(is_admin)
            .bind(points)
            .execute(pool.inner())
            .await
        }
        "quarterly" => {
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, membership_type, membership_expires_at, is_admin, points) VALUES (?, ?, ?, ?, ?, DATE_ADD(NOW(), INTERVAL 90 DAY), ?, ?)"
            )
            .bind(&id)
            .bind(&form.username)
            .bind(&form.email)
            .bind(&password_hash)
            .bind(&membership_type)
            .bind(is_admin)
            .bind(points)
            .execute(pool.inner())
            .await
        }
        "yearly" => {
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, membership_type, membership_expires_at, is_admin, points) VALUES (?, ?, ?, ?, ?, DATE_ADD(NOW(), INTERVAL 365 DAY), ?, ?)"
            )
            .bind(&id)
            .bind(&form.username)
            .bind(&form.email)
            .bind(&password_hash)
            .bind(&membership_type)
            .bind(is_admin)
            .bind(points)
            .execute(pool.inner())
            .await
        }
        "permanent" | "none" | _ => {
            sqlx::query(
                "INSERT INTO users (id, username, email, password_hash, membership_type, is_admin, points) VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(&id)
            .bind(&form.username)
            .bind(&form.email)
            .bind(&password_hash)
            .bind(&membership_type)
            .bind(is_admin)
            .bind(points)
            .execute(pool.inner())
            .await
        }
    };

    match result {
        Ok(_) => Json(ApiResponse::success(id, "用户创建成功")),
        Err(_) => Json(ApiResponse::error("创建失败，用户名或邮箱可能已存在")),
    }
}

#[delete("/users/<id>")]
pub async fn admin_user_delete(
    pool: &State<MySqlPool>,
    admin: AdminUser,
    id: String,
) -> Json<ApiResponse<()>> {
    // 不能删除自己
    if admin.0.id == id {
        return Json(ApiResponse::error("不能删除自己"));
    }

    // 不能删除管理员
    let is_admin: Option<(bool,)> = sqlx::query_as("SELECT is_admin FROM users WHERE id = ?")
        .bind(&id)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

    if let Some((true,)) = is_admin {
        return Json(ApiResponse::error("不能删除管理员账号"));
    }

    let result = sqlx::query("DELETE FROM users WHERE id = ? AND is_admin = FALSE")
        .bind(&id)
        .execute(pool.inner())
        .await;

    match result {
        Ok(r) if r.rows_affected() > 0 => Json(ApiResponse::success((), "删除成功")),
        _ => Json(ApiResponse::error("删除失败")),
    }
}
