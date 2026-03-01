use rocket::{Route, get, post, State};
use rocket::http::{Cookie, CookieJar};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;

use crate::models::*;
use crate::middleware::OptionalUser;
use crate::utils::markdown_to_html;

pub fn routes() -> Vec<Route> {
    routes![
        index,
        article_list,
        article_detail,
        search,
        login_page,
        register_page,
        user_login,
        user_register,
        user_logout,
        pricing_page,
        user_profile,
        help_page,
        faq_page,
        contact_page,
        robots_txt,
    ]
}

#[get("/")]
pub async fn index(
    pool: &State<MySqlPool>,
    user: OptionalUser,
) -> Template {
    // 获取推荐文章
    let featured: Vec<Article> = sqlx::query_as(
        "SELECT * FROM articles WHERE is_visible = TRUE AND is_featured = TRUE ORDER BY created_at DESC LIMIT 6"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    // 获取最新文章
    let latest: Vec<Article> = sqlx::query_as(
        "SELECT * FROM articles WHERE is_visible = TRUE ORDER BY created_at DESC LIMIT 12"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    // 获取分类
    let categories: Vec<Category> = sqlx::query_as(
        "SELECT * FROM categories ORDER BY sort_order"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    Template::render("frontend/index", context! {
        user: user.0,
        featured: featured,
        latest: latest,
        categories: categories,
    })
}

#[get("/articles?<page>&<category>")]
pub async fn article_list(
    pool: &State<MySqlPool>,
    user: OptionalUser,
    page: Option<i32>,
    category: Option<String>,
) -> Template {
    let page = page.unwrap_or(1).max(1);
    let per_page = 12;
    let offset = (page - 1) * per_page;

    let (articles, total): (Vec<Article>, i64) = if let Some(cat_id) = &category {
        let articles: Vec<Article> = sqlx::query_as(
            "SELECT * FROM articles WHERE is_visible = TRUE AND category_id = ? ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(cat_id)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM articles WHERE is_visible = TRUE AND category_id = ?"
        )
        .bind(cat_id)
        .fetch_one(pool.inner())
        .await
        .unwrap_or((0,));

        (articles, count)
    } else {
        let articles: Vec<Article> = sqlx::query_as(
            "SELECT * FROM articles WHERE is_visible = TRUE ORDER BY created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.inner())
        .await
        .unwrap_or_default();

        let (count,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM articles WHERE is_visible = TRUE"
        )
        .fetch_one(pool.inner())
        .await
        .unwrap_or((0,));

        (articles, count)
    };

    let categories: Vec<Category> = sqlx::query_as(
        "SELECT * FROM categories ORDER BY sort_order"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("frontend/articles", context! {
        user: user.0,
        articles: articles,
        categories: categories,
        current_page: page,
        total_pages: total_pages,
        current_category: category,
    })
}

#[get("/article/<id>")]
pub async fn article_detail(
    pool: &State<MySqlPool>,
    user: OptionalUser,
    id: String,
) -> Option<Template> {
    // 增加访问次数
    let _ = sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(&id)
        .execute(pool.inner())
        .await;

    let article: Option<Article> = sqlx::query_as(
        "SELECT * FROM articles WHERE id = ? AND is_visible = TRUE"
    )
    .bind(&id)
    .fetch_optional(pool.inner())
    .await
    .ok()?;

    let article = article?;

    // 获取分类名称
    let category_name: Option<String> = if let Some(cat_id) = &article.category_id {
        sqlx::query_scalar("SELECT name FROM categories WHERE id = ?")
            .bind(cat_id)
            .fetch_optional(pool.inner())
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // 获取平台名称
    let platform_name: Option<String> = if let Some(plat_id) = &article.disk_platform_id {
        sqlx::query_scalar("SELECT name FROM disk_platforms WHERE id = ?")
            .bind(plat_id)
            .fetch_optional(pool.inner())
            .await
            .ok()
            .flatten()
    } else {
        None
    };

    // 获取标签
    let tags: Vec<String> = sqlx::query_scalar(
        "SELECT t.name FROM tags t JOIN article_tags at ON t.id = at.tag_id WHERE at.article_id = ?"
    )
    .bind(&id)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();

    // 判断是否显示资源链接
    let show_resource = user.0.as_ref().map(|u| u.has_valid_membership()).unwrap_or(false);

    let content_html = markdown_to_html(&article.content);

    Some(Template::render("frontend/article_detail", context! {
        user: user.0,
        article: article,
        category_name: category_name,
        platform_name: platform_name,
        tags: tags,
        content_html: content_html,
        show_resource: show_resource,
    }))
}

#[get("/search?<q>&<page>")]
pub async fn search(
    pool: &State<MySqlPool>,
    user: OptionalUser,
    q: Option<String>,
    page: Option<i32>,
) -> Template {
    let query = q.unwrap_or_default();
    let page = page.unwrap_or(1).max(1);
    let per_page = 12;
    let offset = (page - 1) * per_page;

    let (articles, total): (Vec<Article>, i64) = if !query.is_empty() {
        // 使用 FULLTEXT 全文搜索（BOOLEAN MODE 支持更灵活的搜索）
        // 对于中文搜索，需要确保 MySQL/MariaDB 配置了 ngram 分词器
        let search_query = query.split_whitespace()
            .map(|w| format!("+{}*", w))
            .collect::<Vec<_>>()
            .join(" ");
        
        let articles: Vec<Article> = sqlx::query_as(
            "SELECT * FROM articles WHERE is_visible = TRUE AND MATCH(title, content) AGAINST(? IN BOOLEAN MODE) ORDER BY MATCH(title, content) AGAINST(? IN BOOLEAN MODE) DESC, created_at DESC LIMIT ? OFFSET ?"
        )
        .bind(&search_query)
        .bind(&search_query)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool.inner())
        .await
        .unwrap_or_else(|_| {
            // 如果 FULLTEXT 搜索失败（如搜索词太短），回退到 LIKE 搜索
            Vec::new()
        });

        // 如果 FULLTEXT 没有结果，尝试 LIKE 搜索作为回退
        let articles = if articles.is_empty() {
            let like_term = format!("%{}%", query);
            sqlx::query_as(
                "SELECT * FROM articles WHERE is_visible = TRUE AND (title LIKE ? OR content LIKE ?) ORDER BY created_at DESC LIMIT ? OFFSET ?"
            )
            .bind(&like_term)
            .bind(&like_term)
            .bind(per_page)
            .bind(offset)
            .fetch_all(pool.inner())
            .await
            .unwrap_or_default()
        } else {
            articles
        };

        // 计算总数
        let count_result: Result<(i64,), _> = sqlx::query_as(
            "SELECT COUNT(*) FROM articles WHERE is_visible = TRUE AND MATCH(title, content) AGAINST(? IN BOOLEAN MODE)"
        )
        .bind(&search_query)
        .fetch_one(pool.inner())
        .await;

        let count = match count_result {
            Ok((c,)) if c > 0 => c,
            _ => {
                // 回退到 LIKE 计数
                let like_term = format!("%{}%", query);
                sqlx::query_as::<_, (i64,)>(
                    "SELECT COUNT(*) FROM articles WHERE is_visible = TRUE AND (title LIKE ? OR content LIKE ?)"
                )
                .bind(&like_term)
                .bind(&like_term)
                .fetch_one(pool.inner())
                .await
                .unwrap_or((0,)).0
            }
        };

        (articles, count)
    } else {
        (vec![], 0)
    };

    let total_pages = (total as f64 / per_page as f64).ceil() as i32;

    Template::render("frontend/search", context! {
        user: user.0,
        articles: articles,
        query: query,
        current_page: page,
        total_pages: total_pages,
        total: total,
    })
}

#[get("/login")]
pub async fn login_page(user: OptionalUser) -> Template {
    Template::render("frontend/login", context! {
        user: user.0,
    })
}

#[get("/register")]
pub async fn register_page(user: OptionalUser) -> Template {
    Template::render("frontend/register", context! {
        user: user.0,
    })
}

#[post("/login", data = "<form>")]
pub async fn user_login(
    pool: &State<MySqlPool>,
    cookies: &CookieJar<'_>,
    form: Json<LoginRequest>,
) -> Json<ApiResponse<String>> {
    // 查找用户
    let user: Option<User> = sqlx::query_as(
        "SELECT * FROM users WHERE (username = ? OR email = ?) AND is_frozen = FALSE"
    )
    .bind(&form.username)
    .bind(&form.username)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    let user = match user {
        Some(u) => u,
        None => return Json(ApiResponse::error("用户名或密码错误")),
    };

    // 验证密码
    if !bcrypt::verify(&form.password, &user.password_hash).unwrap_or(false) {
        return Json(ApiResponse::error("用户名或密码错误"));
    }

    // 创建会话
    let session_id = uuid::Uuid::new_v4().to_string();
    let token = crate::utils::generate_token();
    
    let _ = sqlx::query(
        "INSERT INTO sessions (id, user_id, token, expires_at) VALUES (?, ?, ?, DATE_ADD(NOW(), INTERVAL 7 DAY))"
    )
    .bind(&session_id)
    .bind(&user.id)
    .bind(&token)
    .execute(pool.inner())
    .await;

    let is_production = std::env::var("ROCKET_ENV").unwrap_or_default() == "production";
    let mut cookie_builder = Cookie::build(("session_token", token))
        .path("/")
        .http_only(true)
        .same_site(rocket::http::SameSite::Strict);

    if is_production {
        cookie_builder = cookie_builder.secure(true);
    }

    cookies.add_private(cookie_builder);

    tracing::info!("用户登录: {}", user.username);
    Json(ApiResponse::success("/".to_string(), "登录成功"))
}

#[post("/register", data = "<form>")]
pub async fn user_register(
    pool: &State<MySqlPool>,
    client_ip: std::net::IpAddr,
    form: Json<RegisterRequest>,
) -> Json<ApiResponse<String>> {
    // 验证邮箱
    if let Err(e) = crate::utils::validate_email(&form.email).await {
        return Json(ApiResponse::error(e));
    }

    // 检查用户名长度
    if form.username.len() < 3 || form.username.len() > 20 {
        return Json(ApiResponse::error("用户名长度需要3-20个字符"));
    }

    // 检查用户名格式（只允许字母、数字、下划线）
    if !form.username.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Json(ApiResponse::error("用户名只能包含字母、数字和下划线"));
    }

    // 检查密码强度
    if let Err(e) = validate_password_strength(&form.password) {
        return Json(ApiResponse::error(e));
    }

    // 检查IP注册限制
    let ip_str = client_ip.to_string();
    let ip_count: Option<(i32,)> = sqlx::query_as(
        "SELECT register_count FROM ip_register_limits WHERE ip_address = ?"
    )
    .bind(&ip_str)
    .fetch_optional(pool.inner())
    .await
    .ok()
    .flatten();

    if let Some((count,)) = ip_count {
        if count >= 5 {
            return Json(ApiResponse::error("该IP注册次数已达上限"));
        }
    }

    // 检查用户名是否存在
    let exists: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE username = ? OR email = ?"
    )
    .bind(&form.username)
    .bind(&form.email)
    .fetch_one(pool.inner())
    .await
    .unwrap_or((0,));

    if exists.0 > 0 {
        return Json(ApiResponse::error("用户名或邮箱已存在"));
    }

    // 创建用户
    let user_id = uuid::Uuid::new_v4().to_string();
    let password_hash = bcrypt::hash(&form.password, bcrypt::DEFAULT_COST).unwrap();

    let result = sqlx::query(
        "INSERT INTO users (id, username, email, password_hash) VALUES (?, ?, ?, ?)"
    )
    .bind(&user_id)
    .bind(&form.username)
    .bind(&form.email)
    .bind(&password_hash)
    .execute(pool.inner())
    .await;

    if result.is_err() {
        return Json(ApiResponse::error("注册失败，请稍后重试"));
    }

    // 更新IP注册计数
    let _ = sqlx::query(
        "INSERT INTO ip_register_limits (ip_address, register_count) VALUES (?, 1) ON DUPLICATE KEY UPDATE register_count = register_count + 1, last_register_at = NOW()"
    )
    .bind(&ip_str)
    .execute(pool.inner())
    .await;

    tracing::info!("新用户注册: {} from {}", form.username, ip_str);
    Json(ApiResponse::success("/login".to_string(), "注册成功，请登录"))
}

/// 密码强度校验
fn validate_password_strength(password: &str) -> Result<(), &'static str> {
    if password.len() < 8 {
        return Err("密码长度至少8个字符");
    }
    if password.len() > 128 {
        return Err("密码长度不能超过128个字符");
    }
    
    let has_lowercase = password.chars().any(|c| c.is_ascii_lowercase());
    let has_uppercase = password.chars().any(|c| c.is_ascii_uppercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special = password.chars().any(|c| !c.is_alphanumeric());
    
    let strength_count = [has_lowercase, has_uppercase, has_digit, has_special]
        .iter()
        .filter(|&&x| x)
        .count();
    
    if strength_count < 2 {
        return Err("密码需包含大小写字母、数字、特殊字符中的至少两种");
    }
    
    // 检查常见弱密码
    let weak_passwords = [
        "password", "12345678", "123456789", "qwerty123", "admin123",
        "letmein", "welcome", "monkey", "dragon", "master",
    ];
    let lower_password = password.to_lowercase();
    for weak in weak_passwords {
        if lower_password.contains(weak) {
            return Err("密码过于简单，请使用更复杂的密码");
        }
    }
    
    Ok(())
}

#[get("/logout")]
pub async fn user_logout(
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
    
    Template::render("frontend/login", context! {
        message: "已退出登录",
    })
}

#[get("/pricing")]
pub async fn pricing_page(
    pool: &State<MySqlPool>,
    user: OptionalUser,
) -> Template {
    let pricing_rows: Vec<crate::models::PricingRow> = sqlx::query_as(
        "SELECT * FROM pricing WHERE is_active = TRUE ORDER BY price"
    )
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let pricing: Vec<Pricing> = pricing_rows.into_iter().map(Pricing::from).collect();

    Template::render("frontend/pricing", context! {
        user: user.0,
        pricing: pricing,
    })
}

#[get("/profile")]
pub async fn user_profile(
    pool: &State<MySqlPool>,
    user: crate::middleware::CurrentUser,
) -> Template {
    let order_rows: Vec<crate::models::OrderRow> = sqlx::query_as(
        "SELECT * FROM orders WHERE user_id = ? ORDER BY created_at DESC LIMIT 20"
    )
    .bind(&user.0.id)
    .fetch_all(pool.inner())
    .await
    .unwrap_or_default();
    
    let orders: Vec<Order> = order_rows.into_iter().map(Order::from).collect();

    Template::render("frontend/profile", context! {
        user: user.0,
        orders: orders,
    })
}


#[get("/help")]
pub async fn help_page(user: OptionalUser) -> Template {
    Template::render("frontend/help", context! {
        user: user.0,
    })
}

#[get("/faq")]
pub async fn faq_page(user: OptionalUser) -> Template {
    Template::render("frontend/faq", context! {
        user: user.0,
    })
}

#[get("/contact")]
pub async fn contact_page(user: OptionalUser) -> Template {
    Template::render("frontend/contact", context! {
        user: user.0,
    })
}

#[get("/robots.txt")]
pub async fn robots_txt() -> &'static str {
    r#"User-agent: *
Disallow: /xuadmin/
Disallow: /api/
Disallow: /login
Disallow: /register
Disallow: /profile

Allow: /
Allow: /articles
Allow: /article/
Allow: /search
Allow: /pricing
Allow: /help
Allow: /faq
Allow: /contact
"#
}
