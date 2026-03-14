#[macro_use]
extern crate rocket;
extern crate lazy_static;

mod config;
mod db;
mod models;
mod routes;
mod middleware;
mod utils;
mod payment;
mod services;

use rocket::fs::FileServer;
use rocket_dyn_templates::Template;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[launch]
async fn rocket() -> _ {
    // 初始化日志
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // 加载环境变量
    dotenvy::dotenv().ok();

    let app_mode = std::env::var("APP_MODE").unwrap_or_else(|_| "all".to_string());
    tracing::info!("启动 Rocket 应用... 模式: {}", app_mode);

    // 初始化数据库
    let pool = db::init_pool().await.expect("数据库连接失败");
    
    // 运行数据库迁移
    db::run_migrations(&pool).await.expect("数据库迁移失败");

    let admin_path = std::env::var("ADMIN_PATH").unwrap_or_else(|_| "xuadmin".to_string());

    let mut app = rocket::build()
        .manage(pool)
        .attach(Template::fairing())
        .mount("/static", FileServer::from("static"))
        .register("/", catchers![not_found, internal_error]);

    match app_mode.as_str() {
        "frontend" => {
            tracing::info!("仅加载前台路由");
            app = app
                .mount("/", routes::frontend::routes())
                .mount("/api", routes::api::routes());
        }
        "admin" => {
            tracing::info!("仅加载后台管理路由, 路径: /{}", admin_path);
            app = app
                .mount(&format!("/{}", admin_path), routes::admin::routes());
        }
        _ => {
            tracing::info!("加载全部路由, 后台路径: /{}", admin_path);
            app = app
                .mount("/", routes::frontend::routes())
                .mount("/api", routes::api::routes())
                .mount(&format!("/{}", admin_path), routes::admin::routes());
        }
    }

    app
}

#[catch(404)]
fn not_found() -> &'static str {
    "页面未找到"
}

#[catch(500)]
fn internal_error() -> &'static str {
    "服务器内部错误"
}
