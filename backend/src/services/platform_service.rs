use sqlx::MySqlPool;
use crate::models::DiskPlatform;

pub async fn list_all(pool: &MySqlPool) -> Vec<DiskPlatform> {
    sqlx::query_as::<_, DiskPlatform>("SELECT * FROM disk_platforms ORDER BY sort_order ASC")
        .fetch_all(pool)
        .await
        .unwrap_or_default()
}

pub async fn create(
    pool: &MySqlPool,
    id: &str,
    name: &str,
    base_url: Option<&str>,
    icon: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO disk_platforms (id, name, base_url, icon) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(name)
        .bind(base_url)
        .bind(icon)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update(
    pool: &MySqlPool,
    id: &str,
    name: &str,
    base_url: Option<&str>,
    icon: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE disk_platforms SET name = ?, base_url = ?, icon = ? WHERE id = ?")
        .bind(name)
        .bind(base_url)
        .bind(icon)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM disk_platforms WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}
