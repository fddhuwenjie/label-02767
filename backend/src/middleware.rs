use rocket::request::{FromRequest, Outcome, Request};
use rocket::http::Status;
use sqlx::MySqlPool;

use crate::models::User;

/// 当前登录用户
pub struct CurrentUser(pub User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for CurrentUser {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let pool = request.guard::<&rocket::State<MySqlPool>>().await.unwrap();
        
        // 从cookie获取token
        let token: String = match request.cookies().get_private("session_token") {
            Some(cookie) => cookie.value().to_string(),
            None => return Outcome::Forward(Status::Unauthorized),
        };

        // 查询会话
        let session: Option<(String,)> = sqlx::query_as(
            "SELECT user_id FROM sessions WHERE token = ? AND expires_at > NOW()"
        )
        .bind(&token)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

        let user_id = match session {
            Some((id,)) => id,
            None => return Outcome::Forward(Status::Unauthorized),
        };

        // 查询用户
        let user: Option<User> = sqlx::query_as(
            "SELECT * FROM users WHERE id = ? AND is_frozen = FALSE"
        )
        .bind(&user_id)
        .fetch_optional(pool.inner())
        .await
        .ok()
        .flatten();

        match user {
            Some(u) => Outcome::Success(CurrentUser(u)),
            None => Outcome::Forward(Status::Unauthorized),
        }
    }
}

/// 管理员用户
pub struct AdminUser(pub User);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AdminUser {
    type Error = &'static str;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let current_user = request.guard::<CurrentUser>().await;
        
        match current_user {
            Outcome::Success(CurrentUser(user)) if user.is_admin => {
                Outcome::Success(AdminUser(user))
            }
            Outcome::Success(_) => Outcome::Forward(Status::Forbidden),
            Outcome::Forward(status) => Outcome::Forward(status),
            Outcome::Error(e) => Outcome::Error(e),
        }
    }
}

/// 可选用户（用于前台页面）
pub struct OptionalUser(pub Option<User>);

#[rocket::async_trait]
impl<'r> FromRequest<'r> for OptionalUser {
    type Error = std::convert::Infallible;

    async fn from_request(request: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        let current_user = request.guard::<CurrentUser>().await;
        
        match current_user {
            Outcome::Success(CurrentUser(user)) => Outcome::Success(OptionalUser(Some(user))),
            _ => Outcome::Success(OptionalUser(None)),
        }
    }
}
