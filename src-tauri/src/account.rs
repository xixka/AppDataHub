//! 账号数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub has_snapshot: bool,
}

impl Account {
    pub fn new(name: String, email: Option<String>, note: Option<String>) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            email,
            note,
            created_at: Utc::now(),
            last_used: None,
            is_active: false,
        }
    }
}

impl From<Account> for AccountMetadata {
    fn from(acc: Account) -> Self {
        Self {
            id: acc.id,
            name: acc.name,
            email: acc.email,
            note: acc.note,
            created_at: acc.created_at,
            last_used: acc.last_used,
            is_active: acc.is_active,
            has_snapshot: false,
        }
    }
}
