use sqlx::MySqlPool;
use crate::models::{Article, ArticleWithDetails, ArticleCreateRequest};

pub async fn find_by_id(pool: &MySqlPool, id: &str) -> Option<Article> {
    sqlx::query_as::<_, Article>("SELECT * FROM articles WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .ok()
        .flatten()
}

pub async fn list_articles(pool: &MySqlPool, page: i64, per_page: i64) -> (Vec<Article>, i64) {
    let total = count_articles(pool).await;
    let offset = (page - 1) * per_page;
    let articles = sqlx::query_as::<_, Article>(
        "SELECT * FROM articles ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();
    (articles, total)
}

pub async fn list_visible_articles(
    pool: &MySqlPool,
    page: i64,
    per_page: i64,
    category: Option<String>,
) -> (Vec<Article>, i64) {
    let offset = (page - 1) * per_page;
    if let Some(ref cat) = category {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM articles a JOIN categories c ON a.category_id = c.id WHERE a.is_visible = true AND c.name = ?",
        )
        .bind(cat)
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let articles = sqlx::query_as::<_, Article>(
            "SELECT a.* FROM articles a JOIN categories c ON a.category_id = c.id WHERE a.is_visible = true AND c.name = ? ORDER BY a.created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(cat)
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        (articles, total)
    } else {
        let total = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM articles WHERE is_visible = true",
        )
        .fetch_one(pool)
        .await
        .unwrap_or(0);

        let articles = sqlx::query_as::<_, Article>(
            "SELECT * FROM articles WHERE is_visible = true ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(per_page)
        .bind(offset)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        (articles, total)
    }
}

pub async fn search_articles(
    pool: &MySqlPool,
    query: &str,
    page: i64,
    per_page: i64,
) -> (Vec<Article>, i64) {
    let pattern = format!("%{}%", query);
    let offset = (page - 1) * per_page;

    let total = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM articles WHERE is_visible = true AND (title LIKE ? OR content LIKE ?)",
    )
    .bind(&pattern)
    .bind(&pattern)
    .fetch_one(pool)
    .await
    .unwrap_or(0);

    let articles = sqlx::query_as::<_, Article>(
        "SELECT * FROM articles WHERE is_visible = true AND (title LIKE ? OR content LIKE ?) ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&pattern)
    .bind(&pattern)
    .bind(per_page)
    .bind(offset)
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    (articles, total)
}

pub async fn get_featured(pool: &MySqlPool, limit: i64) -> Vec<Article> {
    sqlx::query_as::<_, Article>(
        "SELECT * FROM articles WHERE is_visible = true AND is_featured = true ORDER BY created_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn get_latest(pool: &MySqlPool, limit: i64) -> Vec<Article> {
    sqlx::query_as::<_, Article>(
        "SELECT * FROM articles WHERE is_visible = true ORDER BY created_at DESC LIMIT ?",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
}

pub async fn create_article(
    pool: &MySqlPool,
    req: &ArticleCreateRequest,
) -> Result<String, sqlx::Error> {
    let id = uuid::Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO articles (id, title, content, cover_image, category_id, disk_platform_id, disk_link, disk_password, is_visible, is_featured) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.cover_image)
    .bind(&req.category_id)
    .bind(&req.disk_platform_id)
    .bind(&req.disk_link)
    .bind(&req.disk_password)
    .bind(req.is_visible.unwrap_or(true))
    .bind(req.is_featured.unwrap_or(false))
    .execute(pool)
    .await?;

    if let Some(ref tags) = req.tags {
        let _ = sync_article_tags(pool, &id, tags).await;
    }

    Ok(id)
}

pub async fn update_article(
    pool: &MySqlPool,
    id: &str,
    req: &ArticleCreateRequest,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE articles SET title = ?, content = ?, cover_image = ?, category_id = ?, disk_platform_id = ?, disk_link = ?, disk_password = ?, is_visible = ?, is_featured = ? WHERE id = ?",
    )
    .bind(&req.title)
    .bind(&req.content)
    .bind(&req.cover_image)
    .bind(&req.category_id)
    .bind(&req.disk_platform_id)
    .bind(&req.disk_link)
    .bind(&req.disk_password)
    .bind(req.is_visible.unwrap_or(true))
    .bind(req.is_featured.unwrap_or(false))
    .bind(id)
    .execute(pool)
    .await?;

    if let Some(ref tags) = req.tags {
        let _ = sync_article_tags(pool, id, tags).await;
    }

    Ok(())
}

pub async fn delete_article(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM article_tags WHERE article_id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    sqlx::query("DELETE FROM articles WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn increment_view_count(pool: &MySqlPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE articles SET view_count = view_count + 1 WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn get_article_with_details(pool: &MySqlPool, id: &str) -> Option<ArticleWithDetails> {
    #[derive(sqlx::FromRow)]
    struct ArticleDetailRow {
        pub id: String,
        pub title: String,
        pub content: String,
        pub cover_image: Option<String>,
        pub category_name: Option<String>,
        pub platform_name: Option<String>,
        pub disk_link: Option<String>,
        pub disk_password: Option<String>,
        pub is_link_valid: bool,
        pub view_count: i32,
        pub is_visible: bool,
        pub is_featured: bool,
        pub created_at: chrono::DateTime<chrono::Utc>,
    }

    let row = sqlx::query_as::<_, ArticleDetailRow>(
        "SELECT a.id, a.title, a.content, a.cover_image, \
         c.name AS category_name, p.name AS platform_name, \
         a.disk_link, a.disk_password, a.is_link_valid, \
         a.view_count, a.is_visible, a.is_featured, a.created_at \
         FROM articles a \
         LEFT JOIN categories c ON a.category_id = c.id \
         LEFT JOIN disk_platforms p ON a.disk_platform_id = p.id \
         WHERE a.id = ?",
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .ok()
    .flatten()?;

    // Fetch tags
    #[derive(sqlx::FromRow)]
    struct TagName {
        pub name: String,
    }
    let tags = sqlx::query_as::<_, TagName>(
        "SELECT t.name FROM tags t JOIN article_tags at ON t.id = at.tag_id WHERE at.article_id = ?",
    )
    .bind(id)
    .fetch_all(pool)
    .await
    .unwrap_or_default()
    .into_iter()
    .map(|t| t.name)
    .collect();

    Some(ArticleWithDetails {
        id: row.id,
        title: row.title,
        content: row.content,
        cover_image: row.cover_image,
        category_name: row.category_name,
        platform_name: row.platform_name,
        disk_link: row.disk_link,
        disk_password: row.disk_password,
        is_link_valid: row.is_link_valid,
        view_count: row.view_count,
        is_visible: row.is_visible,
        is_featured: row.is_featured,
        tags,
        created_at: row.created_at,
    })
}

pub async fn sync_article_tags(
    pool: &MySqlPool,
    article_id: &str,
    tags_str: &str,
) -> Result<(), sqlx::Error> {
    // Remove existing tags
    sqlx::query("DELETE FROM article_tags WHERE article_id = ?")
        .bind(article_id)
        .execute(pool)
        .await?;

    let tag_names: Vec<&str> = tags_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for tag_name in tag_names {
        // Find or create tag
        let tag_id: Option<String> =
            sqlx::query_scalar("SELECT id FROM tags WHERE name = ?")
                .bind(tag_name)
                .fetch_optional(pool)
                .await?;

        let tag_id = match tag_id {
            Some(id) => id,
            None => {
                let new_id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO tags (id, name) VALUES (?, ?)")
                    .bind(&new_id)
                    .bind(tag_name)
                    .execute(pool)
                    .await?;
                new_id
            }
        };

        sqlx::query("INSERT INTO article_tags (article_id, tag_id) VALUES (?, ?)")
            .bind(article_id)
            .bind(&tag_id)
            .execute(pool)
            .await?;
    }

    Ok(())
}

pub async fn count_articles(pool: &MySqlPool) -> i64 {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM articles")
        .fetch_one(pool)
        .await
        .unwrap_or(0)
}
