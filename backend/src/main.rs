#[macro_use]
extern crate rocket;
extern crate lazy_static;

mod config;
mod db;
mod models;
mod routes;
mod middleware;
mod utils;

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

    tracing::info!("启动 Rocket 应用...");

    // 加载环境变量
    dotenvy::dotenv().ok();

    // 初始化数据库
    let pool = db::init_pool().await.expect("数据库连接失败");
    
    // 运行数据库迁移
    db::run_migrations(&pool).await.expect("数据库迁移失败");

    rocket::build()
        .manage(pool)
        .attach(Template::fairing())
        .mount("/", routes::frontend::routes())
        .mount("/api", routes::api::routes())
        .mount("/xuadmin", routes::admin::routes())
        .mount("/static", FileServer::from("static"))
        .register("/", catchers![not_found, internal_error])
}

#[catch(404)]
fn not_found() -> &'static str {
    "页面未找到"
}

#[catch(500)]
fn internal_error() -> &'static str {
    "服务器内部错误"
}
