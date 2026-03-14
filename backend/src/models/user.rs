use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub is_admin: bool,
    pub membership_type: String,
    pub membership_expires_at: Option<DateTime<Utc>>,
    pub points: i32,
    pub is_frozen: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    pub fn has_valid_membership(&self) -> bool {
        if self.is_admin || self.membership_type == "permanent" {
            return true;
        }
        if self.membership_type == "none" {
            return false;
        }
        if let Some(expires) = self.membership_expires_at {
            return expires > Utc::now();
        }
        false
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct UserUpdateRequest {
    pub email: Option<String>,
    pub membership_type: Option<String>,
    pub membership_expires_at: Option<String>,
    pub points: Option<i32>,
    pub is_frozen: Option<bool>,
    pub is_admin: Option<bool>,
}
