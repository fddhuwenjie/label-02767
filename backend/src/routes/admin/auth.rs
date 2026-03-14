use rocket::{Route, get, post, State};
use rocket::http::{Cookie, CookieJar};
use rocket::serde::json::Json;
use rocket_dyn_templates::{Template, context};
use sqlx::MySqlPool;
use std::net::IpAddr;
use crate::models::*;
use crate::middleware::AdminUser;

pub fn routes() -> Vec<Route> {
    routes![admin_login_page, admin_login, admin_logout]
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
    _admin: AdminUser,
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
