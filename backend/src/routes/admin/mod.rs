pub mod auth;
pub mod articles;
pub mod users;
pub mod orders;
pub mod pricing;
pub mod platforms;
pub mod categories;
pub mod tags;
pub mod upload;
pub mod dashboard;

use rocket::Route;

pub fn routes() -> Vec<Route> {
    let mut routes = Vec::new();
    routes.extend(auth::routes());
    routes.extend(dashboard::routes());
    routes.extend(articles::routes());
    routes.extend(users::routes());
    routes.extend(orders::routes());
    routes.extend(pricing::routes());
    routes.extend(platforms::routes());
    routes.extend(categories::routes());
    routes.extend(tags::routes());
    routes.extend(upload::routes());
    routes
}
