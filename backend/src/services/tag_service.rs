use serde::Serialize;
use sqlx::MySqlPool;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TagWithCount {
    pub id: String,
    pub name: String,
    pub article_count: i64,
    pub created_at: DateTime<Utc>,
}

pub async fn list_with_counts(
    pool: &MySqlPool,
    page: i64,
    per_page: i64,
) -> (Vec<TagWithCount>, i64) {
    let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags")
        .fetch_one(pool)
        .await
        .unwrap_or(0);

    let offset = (page - 1) * per_page;
    let tags = sqlx::query_as::<_, TagWithCount>(
        "SELECT t.id, t.name, COUNT(at.article_id) AS article_count, t.created_at \
         FROM tags t LEFT JOIN article_tags at ON t.id = at.tag_id \
         GROUP BY t.id, t.name, t.created_at \
         ORDER BY t.created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    (tags, total)
}

pub async fn update_name(pool: &MySqlPool, id: &str, new_name: &str) -> Result<(), String> {
    let exists = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM tags WHERE name = ? AND id != ?",
    )
    .bind(new_name)
    .bind(id)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    if exists > 0 {
        return Err("标签名称已存在".to_string());
    }

    sqlx::query("UPDATE tags SET name = ? WHERE id = ?")
        .bind(new_name)
        .bind(id)
        .execute(pool)
        .await
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub async fn delete(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM article_tags WHERE tag_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM tags WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
