use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::env;

pub async fn init_pool() -> Result<MySqlPool, sqlx::Error> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "mysql://root:password@db:3306/rocket_db".to_string());
    
    tracing::info!("连接数据库: {}", database_url.split('@').last().unwrap_or(""));
    
    let pool = MySqlPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await?;
    
    tracing::info!("数据库连接成功");
    Ok(pool)
}

pub async fn run_migrations(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    tracing::info!("运行数据库迁移...");
    
    // 创建用户表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS users (
            id CHAR(36) PRIMARY KEY,
            username VARCHAR(50) NOT NULL UNIQUE,
            email VARCHAR(100) NOT NULL UNIQUE,
            password_hash VARCHAR(255) NOT NULL,
            is_admin BOOLEAN DEFAULT FALSE,
            membership_type ENUM('none', 'monthly', 'quarterly', 'yearly', 'permanent') DEFAULT 'none',
            membership_expires_at DATETIME NULL,
            points INT DEFAULT 0,
            is_frozen BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            INDEX idx_email (email),
            INDEX idx_username (username)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建网盘平台表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS disk_platforms (
            id CHAR(36) PRIMARY KEY,
            name VARCHAR(50) NOT NULL UNIQUE,
            icon VARCHAR(255),
            is_active BOOLEAN DEFAULT TRUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建分类表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS categories (
            id CHAR(36) PRIMARY KEY,
            name VARCHAR(50) NOT NULL UNIQUE,
            sort_order INT DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建标签表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS tags (
            id CHAR(36) PRIMARY KEY,
            name VARCHAR(50) NOT NULL UNIQUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建文章表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS articles (
            id CHAR(36) PRIMARY KEY,
            title VARCHAR(200) NOT NULL,
            content TEXT NOT NULL,
            cover_image VARCHAR(500),
            category_id CHAR(36),
            disk_platform_id CHAR(36),
            disk_link VARCHAR(500),
            disk_password VARCHAR(50),
            is_link_valid BOOLEAN DEFAULT TRUE,
            view_count INT DEFAULT 0,
            is_visible BOOLEAN DEFAULT TRUE,
            is_featured BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
            FOREIGN KEY (category_id) REFERENCES categories(id) ON DELETE SET NULL,
            FOREIGN KEY (disk_platform_id) REFERENCES disk_platforms(id) ON DELETE SET NULL,
            INDEX idx_visible (is_visible),
            INDEX idx_featured (is_featured),
            FULLTEXT INDEX idx_search (title, content)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 添加 cover_image 字段（如果不存在）
    let _ = sqlx::query("ALTER TABLE articles ADD COLUMN cover_image VARCHAR(500) AFTER content")
        .execute(pool).await;

    // 创建文章标签关联表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS article_tags (
            article_id CHAR(36) NOT NULL,
            tag_id CHAR(36) NOT NULL,
            PRIMARY KEY (article_id, tag_id),
            FOREIGN KEY (article_id) REFERENCES articles(id) ON DELETE CASCADE,
            FOREIGN KEY (tag_id) REFERENCES tags(id) ON DELETE CASCADE
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建订单表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS orders (
            id CHAR(36) PRIMARY KEY,
            user_id CHAR(36) NOT NULL,
            membership_type ENUM('monthly', 'quarterly', 'yearly', 'permanent') NOT NULL,
            amount DECIMAL(10, 2) NOT NULL,
            status ENUM('pending', 'paid', 'cancelled', 'refunded') DEFAULT 'pending',
            payment_method VARCHAR(50),
            paid_at DATETIME NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            INDEX idx_user (user_id),
            INDEX idx_status (status)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建定价表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS pricing (
            id CHAR(36) PRIMARY KEY,
            membership_type ENUM('monthly', 'quarterly', 'yearly', 'permanent') NOT NULL UNIQUE,
            price DECIMAL(10, 2) NOT NULL,
            description VARCHAR(200),
            is_active BOOLEAN DEFAULT TRUE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            updated_at DATETIME DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建IP注册限制表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS ip_register_limits (
            ip_address VARCHAR(45) PRIMARY KEY,
            register_count INT DEFAULT 1,
            last_register_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            INDEX idx_last_register (last_register_at)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建登录失败记录表（用于防暴破）
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS login_attempts (
            id CHAR(36) PRIMARY KEY,
            ip_address VARCHAR(45) NOT NULL,
            username VARCHAR(100),
            attempt_type ENUM('admin', 'user') DEFAULT 'user',
            is_success BOOLEAN DEFAULT FALSE,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            INDEX idx_ip (ip_address),
            INDEX idx_username (username),
            INDEX idx_created (created_at)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 创建会话表
    sqlx::query(r#"
        CREATE TABLE IF NOT EXISTS sessions (
            id CHAR(36) PRIMARY KEY,
            user_id CHAR(36) NOT NULL,
            token VARCHAR(255) NOT NULL UNIQUE,
            expires_at DATETIME NOT NULL,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
            INDEX idx_token (token),
            INDEX idx_expires (expires_at)
        ) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4
    "#).execute(pool).await?;

    // 插入默认数据
    init_default_data(pool).await?;

    tracing::info!("数据库迁移完成");
    Ok(())
}

async fn init_default_data(pool: &MySqlPool) -> Result<(), sqlx::Error> {
    // 检查是否已有管理员
    let admin_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin = TRUE")
        .fetch_one(pool)
        .await?;
    
    if admin_exists.0 == 0 {
        // 创建默认管理员
        let admin_id = uuid::Uuid::new_v4().to_string();
        let password_hash = bcrypt::hash("admin123", bcrypt::DEFAULT_COST).unwrap();
        
        sqlx::query(r#"
            INSERT INTO users (id, username, email, password_hash, is_admin, membership_type)
            VALUES (?, 'admin', 'admin@example.com', ?, TRUE, 'permanent')
        "#)
        .bind(&admin_id)
        .bind(&password_hash)
        .execute(pool)
        .await?;
        
        tracing::info!("创建默认管理员账号: admin / admin123");
    }

    // 检查是否已有定价
    let pricing_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM pricing")
        .fetch_one(pool)
        .await?;
    
    if pricing_exists.0 == 0 {
        // 插入默认定价
        let prices = [
            ("monthly", 29.00, "月度会员 - 30天全站资源无限下载"),
            ("quarterly", 79.00, "季度会员 - 90天全站资源无限下载"),
            ("yearly", 199.00, "年度会员 - 365天全站资源无限下载"),
            ("permanent", 399.00, "永久会员 - 终身有效，一次购买永久使用"),
        ];
        
        for (mtype, price, desc) in prices {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(r#"
                INSERT INTO pricing (id, membership_type, price, description)
                VALUES (?, ?, ?, ?)
            "#)
            .bind(&id)
            .bind(mtype)
            .bind(price)
            .bind(desc)
            .execute(pool)
            .await?;
        }
        tracing::info!("创建默认定价方案");
    }

    // 检查是否已有网盘平台
    let platform_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM disk_platforms")
        .fetch_one(pool)
        .await?;
    
    let mut platform_ids: Vec<String> = Vec::new();
    if platform_exists.0 == 0 {
        let platforms = ["百度网盘", "夸克网盘", "阿里云盘", "天翼云盘", "123云盘", "迅雷云盘"];
        for name in platforms {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO disk_platforms (id, name) VALUES (?, ?)")
                .bind(&id)
                .bind(name)
                .execute(pool)
                .await?;
            platform_ids.push(id);
        }
        tracing::info!("创建默认网盘平台");
    } else {
        // 获取已有平台ID
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM disk_platforms LIMIT 6")
            .fetch_all(pool)
            .await?;
        for row in rows {
            platform_ids.push(row.0);
        }
    }

    // 检查是否已有分类
    let category_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM categories")
        .fetch_one(pool)
        .await?;
    
    let mut category_ids: Vec<String> = Vec::new();
    if category_exists.0 == 0 {
        let categories = [
            ("设计素材", 1),
            ("开发教程", 2),
            ("办公模板", 3),
            ("影视资源", 4),
            ("电子书籍", 5),
            ("软件工具", 6),
        ];
        for (name, sort) in categories {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query("INSERT INTO categories (id, name, sort_order) VALUES (?, ?, ?)")
                .bind(&id)
                .bind(name)
                .bind(sort)
                .execute(pool)
                .await?;
            category_ids.push(id);
        }
        tracing::info!("创建默认分类");
    } else {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM categories ORDER BY sort_order LIMIT 6")
            .fetch_all(pool)
            .await?;
        for row in rows {
            category_ids.push(row.0);
        }
    }

    // 检查是否已有文章
    let article_exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM articles")
        .fetch_one(pool)
        .await?;
    
    if article_exists.0 == 0 && !category_ids.is_empty() && !platform_ids.is_empty() {
        // 创建测试文章 (title, content, cover_image, cat_idx, plat_idx, link, pwd, visible, featured, views)
        let articles = [
            (
                "2024最新UI设计规范完整版 - 包含组件库和设计系统",
                "## 资源介绍\n\n这是一套完整的UI设计规范，包含了最新的设计趋势和组件库。\n\n### 包含内容\n\n- 完整的设计系统文档\n- Figma组件库源文件\n- Sketch组件库源文件\n- 设计规范PDF文档\n- 配色方案和字体规范\n\n### 适用人群\n\n- UI/UX设计师\n- 产品经理\n- 前端开发工程师",
                "https://images.unsplash.com/photo-1561070791-2526d30994b5?w=800&h=600&fit=crop",
                0, 0, "https://pan.baidu.com/s/1example1", "abc1", true, true, 156
            ),
            (
                "Python全栈开发实战教程 - 从入门到精通",
                "## 课程简介\n\n本教程涵盖Python全栈开发的所有核心知识点，从基础语法到高级应用。\n\n### 课程大纲\n\n1. Python基础语法\n2. 面向对象编程\n3. Web开发框架Django/Flask\n4. 数据库操作\n5. RESTful API设计",
                "https://images.unsplash.com/photo-1526379095098-d400fd0bf935?w=800&h=600&fit=crop",
                1, 1, "https://pan.quark.cn/s/example2", "xyz2", true, true, 289
            ),
            (
                "商务PPT模板合集 - 500+精品模板",
                "## 模板介绍\n\n精选500+商务PPT模板，涵盖各种场景和风格。\n\n### 模板分类\n\n- 工作汇报\n- 商业计划书\n- 产品发布\n- 年终总结",
                "https://images.unsplash.com/photo-1586281380349-632531db7ed4?w=800&h=600&fit=crop",
                2, 2, "https://www.aliyundrive.com/s/example3", "ppt3", true, false, 423
            ),
            (
                "4K高清纪录片合集 - BBC精选系列",
                "## 资源介绍\n\n精选BBC出品的高质量纪录片，4K超清画质。\n\n### 包含系列\n\n- 地球脉动 Planet Earth\n- 蓝色星球 Blue Planet\n- 人类星球 Human Planet",
                "https://images.unsplash.com/photo-1536440136628-849c177e76a1?w=800&h=600&fit=crop",
                3, 3, "https://cloud.189.cn/t/example4", "bbc4", true, false, 567
            ),
            (
                "计算机科学经典书籍合集 - PDF高清版",
                "## 书籍列表\n\n精选计算机科学领域的经典著作，PDF高清扫描版。\n\n### 包含书籍\n\n- 《算法导论》第三版\n- 《深入理解计算机系统》\n- 《设计模式》",
                "https://images.unsplash.com/photo-1532012197267-da84d127e765?w=800&h=600&fit=crop",
                4, 4, "https://www.123pan.com/s/example5", "book5", true, true, 892
            ),
            (
                "Adobe全家桶2024破解版 - Win/Mac双版本",
                "## 软件介绍\n\nAdobe Creative Cloud 2024全套软件，包含Win和Mac双版本。\n\n### 包含软件\n\n- Photoshop 2024\n- Illustrator 2024\n- Premiere Pro 2024",
                "https://images.unsplash.com/photo-1626785774625-ddcddc3445e9?w=800&h=600&fit=crop",
                5, 0, "https://pan.baidu.com/s/example6", "soft6", true, false, 1234
            ),
            (
                "Figma高级技巧教程 - 提升设计效率",
                "## 教程介绍\n\n深入学习Figma的高级功能和技巧，大幅提升设计效率。\n\n### 课程内容\n\n- Auto Layout高级用法\n- 组件变体设计\n- 设计系统搭建",
                "https://images.unsplash.com/photo-1581291518633-83b4ebd1d83e?w=800&h=600&fit=crop",
                1, 1, "https://pan.quark.cn/s/example7", "fig7", true, false, 345
            ),
            (
                "React + TypeScript实战项目源码",
                "## 项目介绍\n\n完整的React + TypeScript企业级项目源码，包含前后端。\n\n### 技术栈\n\n- React 18\n- TypeScript 5\n- Ant Design 5",
                "https://images.unsplash.com/photo-1633356122544-f134324a6cee?w=800&h=600&fit=crop",
                1, 2, "https://www.aliyundrive.com/s/example8", "react8", true, true, 678
            ),
            (
                "摄影后期调色预设合集 - Lightroom/PS通用",
                "## 预设介绍\n\n专业摄影师调色预设，适用于各种拍摄场景。\n\n### 预设分类\n\n- 人像美肤\n- 风景风光\n- 城市街拍",
                "https://images.unsplash.com/photo-1542038784456-1ea8e935640e?w=800&h=600&fit=crop",
                0, 3, "https://cloud.189.cn/t/example9", "lr9", true, false, 456
            ),
            (
                "Excel数据分析实战案例 - 含源文件",
                "## 课程介绍\n\n通过真实案例学习Excel数据分析，提升职场竞争力。\n\n### 案例内容\n\n- 销售数据分析\n- 财务报表制作",
                "https://images.unsplash.com/photo-1460925895917-afdab827c52f?w=800&h=600&fit=crop",
                2, 4, "https://www.123pan.com/s/example10", "excel10", true, false, 234
            ),
        ];

        for (title, content, cover, cat_idx, plat_idx, link, pwd, visible, featured, views) in articles {
            let id = uuid::Uuid::new_v4().to_string();
            let cat_id = category_ids.get(cat_idx).cloned();
            let plat_id = platform_ids.get(plat_idx).cloned();
            
            sqlx::query(r#"
                INSERT INTO articles (id, title, content, cover_image, category_id, disk_platform_id, disk_link, disk_password, is_visible, is_featured, view_count)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#)
            .bind(&id)
            .bind(title)
            .bind(content)
            .bind(cover)
            .bind(&cat_id)
            .bind(&plat_id)
            .bind(link)
            .bind(pwd)
            .bind(visible)
            .bind(featured)
            .bind(views)
            .execute(pool)
            .await?;
        }
        tracing::info!("创建测试文章数据");
    }

    // 创建测试用户
    let user_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users WHERE is_admin = FALSE")
        .fetch_one(pool)
        .await?;
    
    if user_count.0 == 0 {
        let test_users = [
            ("zhangsan", "zhangsan@example.com", "monthly"),
            ("lisi", "lisi@example.com", "quarterly"),
            ("wangwu", "wangwu@example.com", "yearly"),
            ("testuser", "test@example.com", "none"),
        ];
        
        let password_hash = bcrypt::hash("123456", bcrypt::DEFAULT_COST).unwrap();
        
        for (username, email, membership) in test_users {
            let id = uuid::Uuid::new_v4().to_string();
            sqlx::query(r#"
                INSERT INTO users (id, username, email, password_hash, membership_type, points)
                VALUES (?, ?, ?, ?, ?, ?)
            "#)
            .bind(&id)
            .bind(username)
            .bind(email)
            .bind(&password_hash)
            .bind(membership)
            .bind(100)
            .execute(pool)
            .await?;
        }
        tracing::info!("创建测试用户数据");
    }

    Ok(())
}
