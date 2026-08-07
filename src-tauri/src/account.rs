//! 账号数据模型

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 账号完整数据 (内部)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub note: Option<String>,
    pub plugin_id: String,
    pub bound_machine_id: Option<String>,
    /// 加密的 token 快照 (可选, 用于保存登录态)
    pub token_enc: Option<String>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub has_snapshot: bool,
}

/// 账号元数据 (返回给前端)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountMetadata {
    pub id: String,
    pub name: String,
    pub email: Option<String>,
    pub note: Option<String>,
    pub plugin_id: String,
    pub has_snapshot: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub is_active: bool,
}

impl Account {
    pub fn new(name: String, email: Option<String>, note: Option<String>, plugin_id: String) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            email,
            note,
            plugin_id,
            bound_machine_id: None,
            token_enc: None,
            created_at: Utc::now(),
            last_used_at: None,
            is_active: false,
            has_snapshot: false,
        }
    }
}

impl From<&Account> for AccountMetadata {
    fn from(acc: &Account) -> Self {
        Self {
            id: acc.id.clone(),
            name: acc.name.clone(),
            email: acc.email.clone(),
            note: acc.note.clone(),
            plugin_id: acc.plugin_id.clone(),
            has_snapshot: acc.has_snapshot,
            created_at: acc.created_at,
            last_used_at: acc.last_used_at,
            is_active: acc.is_active,
        }
    }
}
