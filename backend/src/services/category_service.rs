use sqlx::MySqlPool;
use crate::models::Category;

pub async fn list_all(pool: &MySqlPool) -> Vec<Category> {
    sqlx::query_as::<_, Category>("SELECT * FROM categories ORDER BY sort_order ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

pub async fn create(
    pool: &MySqlPool,
    id: &str,
    name: &str,
    sort_order: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO categories (id, name, sort_order) VALUES (?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(sort_order)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM categories WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
