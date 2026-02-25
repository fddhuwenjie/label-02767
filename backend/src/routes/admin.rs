use rocket::{Route, get, post, put, delete, State};
use rocket::http::{Cookie, CookieJar};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use std::net::IpAddr;

use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![
        admin_login_page,
        admin_login,
        admin_logout,
        admin_dashboard,
        admin_articles,
        admin_article_create_page,
        admin_article_create,
        admin_article_edit_page,
        admin_article_update,
        admin_article_delete,
        admin_users,
        admin_user_update,
        admin_user_create,
        admin_user_delete,
        admin_orders,
        admin_pricing,
        admin_pricing_update,
        admin_platforms,
        admin_platform_create,
        admin_platform_update,
        admin_platform_delete,
        admin_categories,
        admin_category_create,
        admin_category_delete,
    ]
}

#[get("/login")]
pub async fn admin_login_page() -> Template {
    Template::render("admin/login", context! {})
}

#[post("/login", data = "<form>")]
pub async fn admin_login(
    pool: &State<MySqlPool>,
    cookies: &CookieJar<'_>,
    client_ip: IpAddr,
    form: Json<LoginRequest>,
) -> Json<ApiResponse<String>> {
    let ip_str = client_ip.to_string();
    
    // 检查IP是否被锁定（15分钟内失败超过5次）
    let (ip_fail_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM login_attempts WHERE ip_address = ? AND attempt_type = 'admin' AND is_success = FALSE AND created_at > DATE_SUB(NOW(), INTERVAL 15 MINUTE)"
    )
    .bind(&ip_str)
    .fetch_one(pool.inner())
    .await
    .unwrap_or((0,));

    if ip_fail_count >= 5 {
        tracing::warn!("IP {} 登录尝试过于频繁，已被临时锁定", ip_str);
        return Json(ApiResponse::error("登录尝试过于频繁，请15分钟后再试"));
    }

    // 检查账户是否被锁定（15分钟内该账户失败超过3次）
    let (account_fail_count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM login_attempts WHERE username = ? AND attempt_type = 'admin' AND is_success = FALSE AND created_at > DATE_SUB(NOW(), INTERVAL 15 MINUTE)"
    )
    .bind(&form.username)
    .fetch_one(pool.inner())
    .await
    .unwrap_or((0,));

    if account_fail_count >= 3 {
        // 记录失败尝试
        let _ = sqlx::query(
            "INSERT INTO login_attempts (id, ip_address, username, attempt_type, is_success) VALUES (?, ?, ?, 'admin', FALSE)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&ip_str)
        .bind(&form.username)
        .execute(pool.inner())
        .await;
        
        tracing::warn!("账户 {} 登录失败次数过多，已被临时锁定", form.username);
        return Json(ApiResponse::error("该账户已被临时锁定，请15分钟后再试"));
    }

    // 查找管理员
    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE (username = ? OR email = ?) AND is_admin = TRUE"
    )
    .bind(&form.username)
    .bind(&form.username)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    let user = match user {
        Some(u) => u,
        None => {
            // 记录失败尝试
            let _ = sqlx::query(
                "INSERT INTO login_attempts (id, ip_address, username, attempt_type, is_success) VALUES (?, ?, ?, 'admin', FALSE)"
            )
            .bind(uuid::Uuid::new_v4().to_string())
            .bind(&ip_str)
            .bind(&form.username)
            .execute(pool.inner())
            .await;
            
            tracing::warn!("管理员登录失败: {} from {}", form.username, ip_str);
            return Json(ApiResponse::error("用户名或密码错误"));
        }
    };

    // 验证密码
    if !bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false) {
        // 记录失败尝试
        let _ = sqlx::query(
            "INSERT INTO login_attempts (id, ip_address, username, attempt_type, is_success) VALUES (?, ?, ?, 'admin', FALSE)"
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&ip_str)
        .bind(&form.username)
        .execute(pool.inner())
        .await;
        
        tracing::warn!("管理员登录密码错误: {} from {}", form.username, ip_str);
        return Json(ApiResponse::error("用户名或密码错误"));
    }

    // 记录成功登录
    let _ = sqlx::query(
        "INSERT INTO login_attempts (id, ip_address, username, attempt_type, is_success) VALUES (?, ?, ?, 'admin', TRUE)"
    )
    .bind(uuid::Uuid::new_v4().to_string())
    .bind(&ip_str)
    .bind(&form.username)
    .execute(pool.inner())
    .await;

    // 创建会话
    let session_id = uuid::Uuid::new_v4().to_string();
    let token = crate::utils::generate_token();
    
    let _ = sqlx::query(
        "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?, ?, ?, DATE_ADD(NOW(), INTERVAL 1 DAY))"
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(&token)
    .execute(pool.inner())
    .await;

    // 设置安全Cookie - 生产环境应启用Secure
    let is_production = std::env::var("ROCKET_ENV").unwrap_or_default() == "production";
    let mut cookie_builder = Cookie::build(("session_token", token))
        .path("/")
        .http_only(true)
        .same_site(rocket::http::SameSite::Strict);
    
    if is_production {
        cookie_builder = cookie_builder.secure(true);
    }
    
    cookies.add_private(cookie_builder);

    tracing::info!("管理员登录成功: {} from {}", user.username, ip_str);
    Json(ApiResponse::success("/xuadmin/dashboard".to_string(), "登录成功"))
}

#[get("/logout")]
pub async fn admin_logout(
    pool: &State<MySqlPool>,
    cookies: &CookieJar<'_>,
) -> Template {
    if let Some(cookie) = cookies.get_private("session_token") {
        let _ = sqlx::query("DELETE FROM sessions WHERE token = ?")
            .bind(cookie.value())
            .execute(pool.inner())
            .await;
    }
    cookies.remove_private("session_token");
    Template::render("admin/login", context! { message: "已退出登录" })
}

#[get("/dashboard")]
pub async fn admin_dashboard(
    pool: &State<MySqlPool>,
    admin: AdminUser,
) -> Template {
    // 统计数据
    let (total_users,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let (total_articles,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let (total_orders,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM orders WHERE status = 'paid'")
        .fetch_one(pool.inner()).await.unwrap_or((0,));
    
    let total_revenue: f64 = sqlx::query_scalar::<_, rust_decimal::Decimal>("SELECT COALESCE(SUM(amount), 0) FROM orders WHERE status = 'paid'")
        .fetch_one(pool.inner())
        .await
        .map(|d| d.to_string().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0);

    // 最近订单
    let recent_order_rows: Vec<crate::models::OrderRow> = sqlx::query_as(
        "SELECT * FROM orders ORDER BY created_at DESC LIMIT 10"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let recent_orders: Vec<Order> = recent_order_rows.into_iter().map(Order::from).collect();

    // 月度收入统计 - 使用自定义结构体
    #[derive(sqlx::FromRow)]
    struct MonthlyRevenueRow {
        month: Option<String>,
        revenue: rust_decimal::Decimal,
    }
    
    let monthly_rows: Vec<MonthlyRevenueRow> = sqlx::query_as(
        "SELECT DATE_FORMAT(paid_at, '%Y-%m') as month, COALESCE(SUM(amount), 0) as revenue FROM orders WHERE status = 'paid' AND paid_at >= DATE_SUB(NOW(), INTERVAL 6 MONTH) GROUP BY month ORDER BY month"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let monthly_revenue: Vec<(String, f64)> = monthly_rows
        .into_iter()
        .map(|r| (r.month.unwrap_or_default(), r.revenue.to_string().parse::<f64>().unwrap_or(0.0)))
        .collect();

    Template::render("admin/dashboard", context! {
        user: admin.0,
        total_users: total_users,
        total_articles: total_articles,
        total_orders: total_orders,
        total_revenue: total_revenue,
        recent_orders: recent_orders,
        monthly_revenue: monthly_revenue,
    })
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
                for tag_name in tags {
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
                for tag_name in tags {
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
    form: Json<crate::models::RegisterRequest>,
) -> Json<ApiResponse<String>> {
    let id = uuid::Uuid::new_v4().to_string();
    let password_hash = bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).unwrap();

    let result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)"
    )
    .bind(&id)
    .bind(&form.username)
    .bind(&form.email)
    .bind(&password_hash)
    .execute(pool.inner())
    .await;

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

#[derive(serde::Deserialize)]
pub struct PlatformRequest {
    pub name: String,
    pub icon: Option<String>,
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

#[derive(serde::Deserialize)]
pub struct CategoryRequest {
    pub name: String,
    pub sort_order: Option<i32>,
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
