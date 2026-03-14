use sqlx::MySqlPool;
use crate::models::{User, UserUpdateRequest};

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Option<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn find_by_username(pool: &MySqlPool, username: &str) -> Option<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn find_by_username_or_email(pool: &MySqlPool, username: &str) -> Option<User> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE username = ? OR email = ?")
        .bind(username)
        .bind(username)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn create_user(
    pool: &MySqlPool,
    id: &str,
    username: &str,
    email: &str,
    password_hash: &str,
    is_admin: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO users (id, username, email, password_hash, is_admin) VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(username)
    .bind(email)
    .bind(password_hash)
    .bind(is_admin)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user(
    pool: &MySqlPool,
    id: &str,
    req: &UserUpdateRequest,
) -> Result<(), String> {
    let mut set_clauses = Vec::new();
    let mut values: Vec<String> = Vec::new();

    if let Some(ref email) = req.email {
        set_clauses.push("email = ?".to_string());
        values.push(email.clone());
    }
    if let Some(ref membership_type) = req.membership_type {
        set_clauses.push("membership_type = ?".to_string());
        values.push(membership_type.clone());
    }
    if let Some(ref membership_expires_at) = req.membership_expires_at {
        set_clauses.push("membership_expires_at = ?".to_string());
        values.push(membership_expires_at.clone());
    }
    if let Some(points) = req.points {
        set_clauses.push("points = ?".to_string());
        values.push(points.to_string());
    }
    if let Some(is_frozen) = req.is_frozen {
        set_clauses.push("is_frozen = ?".to_string());
        values.push(if is_frozen { "1".to_string() } else { "0".to_string() });
    }
    if let Some(is_admin) = req.is_admin {
        set_clauses.push("is_admin = ?".to_string());
        values.push(if is_admin { "1".to_string() } else { "0".to_string() });
    }

    if set_clauses.is_empty() {
        return Err("没有需要更新的字段".to_string());
    }

    let sql = format!("UPDATE users SET {} WHERE id = ?", set_clauses.join(", "));
    let mut query = sqlx::query(&sql);
    for v in &values {
        query = query.bind(v);
    }
    query = query.bind(id);

    query.execute(pool).await.map_err(|e| e.to_string())?;
    Ok(())
}

pub async fn delete_user(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_users(pool: &MySqlPool, page: i64, per_page: i64) -> (Vec<User>, i64) {
    let total = count_users(pool).await;
    let offset = (page - 1) * per_page;
    let users = sqlx::query_as::<_, User>("SELECT * FROM users ORDER BY created_at DESC LIMIT ? OFFSET ?")
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
    (users, total)
}

pub async fn count_users(pool: &MySqlPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}

pub async fn record_login_attempt(
    pool: &MySqlPool,
    ip: &str,
    attempt_type: &str,
    is_success: bool,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO login_attempts (ip_address, attempt_type, is_success) VALUES (?, ?, ?)",
    )
    .bind(ip)
    .bind(attempt_type)
    .bind(is_success)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn count_failed_attempts(pool: &MySqlPool, ip: &str, attempt_type: &str) -> i64 {
    sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM login_attempts WHERE ip_address = ? AND attempt_type = ? AND is_success = false AND created_at > DATE_SUB(NOW(), INTERVAL 15 MINUTE)",
    )
    .bind(ip)
    .bind(attempt_type)
    .fetch_one(pool)
    .await
    .unwrap_or(0)
}

pub async fn update_membership(
    pool: &MySqlPool,
    user_id: &str,
    membership_type: &str,
    expires_at: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE users SET membership_type = ?, membership_expires_at = ? WHERE id = ?",
    )
    .bind(membership_type)
    .bind(expires_at)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(())
}
