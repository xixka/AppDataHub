//! 账号仓库 — 本地持久化存储
//!
//! 数据文件: {app_data_dir}/accounts.json
//! 配置快照: {app_data_dir}/snapshots/{account_id}/

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::account::{Account, AccountMetadata};
use crate::config::{AppProfile, ProfileConfig};

#[derive(Debug, Error)]
pub enum StoreError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("账号不存在: {0}")]
    NotFound(String),
    #[error("账号名已存在: {0}")]
    DuplicateName(String),
    #[error("操作失败: {0}")]
    Other(String),
}

/// 仓库数据结构
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreData {
    pub accounts: Vec<Account>,
    pub active_account_id: Option<String>,
    pub encrypted: bool,
    pub encryption_key: Option<String>,
}

/// 账号仓库
pub struct Store {
    data: StoreData,
    #[allow(dead_code)]
    data_dir: PathBuf,
    data_file: PathBuf,
    snapshots_dir: PathBuf,
    profiles_file: PathBuf,
    profile: AppProfile,
}

impl Store {
    pub fn new(data_dir: PathBuf, profile: AppProfile) -> Self {
        let data_file = data_dir.join("accounts.json");
        let snapshots_dir = data_dir.join("snapshots");
        let profiles_file = data_dir.join("profiles.json");
        let _ = fs::create_dir_all(&data_dir);
        Self {
            data: StoreData::default(),
            data_dir,
            data_file,
            snapshots_dir,
            profiles_file,
            profile,
        }
    }

    pub fn load(&mut self) -> Result<(), StoreError> {
        if self.data_file.exists() {
            let content = fs::read_to_string(&self.data_file)?;
            self.data = serde_json::from_str(&content)?;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), StoreError> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.data_file, content)?;
        Ok(())
    }

    pub fn list_accounts(&self) -> Vec<AccountMetadata> {
        self.data
            .accounts
            .iter()
            .map(|a| {
                let mut m: AccountMetadata = a.clone().into();
                m.has_snapshot = self.snapshots_dir.join(&a.id).exists();
                m
            })
            .collect()
    }

    pub fn add_account(
        &mut self,
        name: String,
        email: Option<String>,
        note: Option<String>,
    ) -> Result<Account, StoreError> {
        if self.data.accounts.iter().any(|a| a.name == name) {
            return Err(StoreError::DuplicateName(name));
        }
        let account = Account::new(name, email, note);
        self.data.accounts.push(account.clone());
        self.save()?;
        Ok(account)
    }

    pub fn delete_account(&mut self, id: &str) -> Result<(), StoreError> {
        let idx = self
            .data
            .accounts
            .iter()
            .position(|a| a.id.as_str() == id)
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        self.data.accounts.remove(idx);
        if self.data.active_account_id.as_deref() == Some(id) {
            self.data.active_account_id = None;
        }
        let snapshot = self.snapshots_dir.join(id);
        if snapshot.exists() {
            let _ = fs::remove_dir_all(&snapshot);
        }
        self.save()?;
        Ok(())
    }

    pub fn update_account(
        &mut self,
        id: &str,
        name: Option<String>,
        email: Option<Option<String>>,
        note: Option<Option<String>>,
    ) -> Result<Account, StoreError> {
        if let Some(ref name) = name {
            if self
                .data
                .accounts
                .iter()
                .any(|a| a.id != id && a.name == name.as_str())
            {
                return Err(StoreError::DuplicateName(name.clone()));
            }
        }
        let account = self
            .data
            .accounts
            .iter_mut()
            .find(|a| a.id.as_str() == id)
            .ok_or_else(|| StoreError::NotFound(id.into()))?;
        if let Some(name) = name {
            account.name = name;
        }
        if let Some(email) = email {
            account.email = email;
        }
        if let Some(note) = note {
            account.note = note;
        }
        let updated = account.clone();
        self.save()?;
        Ok(updated)
    }

    /// 保存当前应用配置快照到指定账号
    pub fn save_current_snapshot(&mut self, id: &str) -> Result<(), StoreError> {
        if !self.data.accounts.iter().any(|a| a.id.as_str() == id) {
            return Err(StoreError::NotFound(id.into()));
        }
        let snapshot_dir = self.snapshots_dir.join(id);
        if snapshot_dir.exists() {
            let _ = fs::remove_dir_all(&snapshot_dir);
        }
        self.profile
            .backup_to(&snapshot_dir)
            .map_err(|e| StoreError::Other(e.to_string()))?;
        Ok(())
    }

    /// 切换到指定账号
    pub fn switch_account(&mut self, id: &str) -> Result<(), StoreError> {
        if !self.data.accounts.iter().any(|a| a.id.as_str() == id) {
            return Err(StoreError::NotFound(id.into()));
        }

        // 步骤 1: 如果有当前激活账号, 先备份当前配置
        if let Some(old_id) = self.data.active_account_id.clone() {
            if old_id != id {
                let old_snapshot = self.snapshots_dir.join(&old_id);
                if old_snapshot.exists() {
                    let _ = fs::remove_dir_all(&old_snapshot);
                }
                self.profile
                    .backup_to(&old_snapshot)
                    .map_err(|e| StoreError::Other(e.to_string()))?;
                if let Some(acc) = self
                    .data
                    .accounts
                    .iter_mut()
                    .find(|a| a.id.as_str() == old_id.as_str())
                {
                    acc.last_used = Some(Utc::now());
                    acc.is_active = false;
                }
            }
        }

        // 步骤 2: 恢复目标账号快照
        let target_snapshot = self.snapshots_dir.join(id);
        if target_snapshot.exists() {
            self.profile
                .restore_from(&target_snapshot)
                .map_err(|e| StoreError::Other(e.to_string()))?;
        } else {
            // 无快照时, 仅备份当前配置 (首次切换)
            let _ = fs::create_dir_all(&target_snapshot);
            self.profile
                .backup_to(&target_snapshot)
                .map_err(|e| StoreError::Other(e.to_string()))?;
        }

        // 步骤 3: 更新激活状态
        for acc in &mut self.data.accounts {
            acc.is_active = acc.id.as_str() == id;
        }
        if let Some(acc) = self
            .data
            .accounts
            .iter_mut()
            .find(|a| a.id.as_str() == id)
        {
            acc.last_used = Some(Utc::now());
        }
        self.data.active_account_id = Some(id.into());
        self.save()?;
        Ok(())
    }

    pub fn get_active_account(&self) -> Option<&Account> {
        self.data
            .active_account_id
            .as_ref()
            .and_then(|id| self.data.accounts.iter().find(|a| a.id.as_str() == id.as_str()))
    }

    pub fn get_profile_info(&self) -> (&PathBuf, Option<&PathBuf>, bool) {
        (
            &self.profile.config_dir,
            self.profile.user_dir.as_ref(),
            self.profile.exists(),
        )
    }

    pub fn set_profile_paths(&mut self, config_dir: PathBuf, user_dir: Option<PathBuf>) {
        self.profile = AppProfile::custom(config_dir, user_dir);
    }

    pub fn is_app_running(&self) -> bool {
        self.profile.is_running()
    }

    /// 从配置文件加载所有可用 profiles
    pub fn list_profiles(&self) -> Vec<ProfileConfig> {
        AppProfile::load_all(&self.profiles_file).unwrap_or_default()
    }

    /// 切换当前使用的 profile (按索引)
    pub fn select_profile(&mut self, index: usize) -> Result<(), StoreError> {
        let profiles = self.list_profiles();
        let cfg = profiles
            .get(index)
            .ok_or_else(|| StoreError::Other(format!("profile 索引 {} 不存在", index)))?;
        self.profile = AppProfile::from_config(cfg)
            .map_err(|e| StoreError::Other(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppProfile;
    use std::path::PathBuf;

    fn temp_dir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("appdatahub_test_{}", uuid::Uuid::new_v4()));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn test_add_and_list() {
        let dir = temp_dir();
        let profile = AppProfile::custom(dir.join("cfg"), None);
        let mut store = Store::new(dir.join("store"), profile);
        let acc = store
            .add_account("Test1".into(), Some("e@t.com".into()), None)
            .unwrap();
        let list = store.list_accounts();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "Test1");
        assert_eq!(list[0].id, acc.id);
    }

    #[test]
    fn test_duplicate_name() {
        let dir = temp_dir();
        let profile = AppProfile::custom(dir.join("cfg"), None);
        let mut store = Store::new(dir.join("store"), profile);
        store.add_account("Dup".into(), None, None).unwrap();
        assert!(store.add_account("Dup".into(), None, None).is_err());
    }

    #[test]
    fn test_delete_account() {
        let dir = temp_dir();
        let profile = AppProfile::custom(dir.join("cfg"), None);
        let mut store = Store::new(dir.join("store"), profile);
        let acc = store.add_account("Del".into(), None, None).unwrap();
        store.delete_account(&acc.id).unwrap();
        assert_eq!(store.list_accounts().len(), 0);
    }

    #[test]
    fn test_update_account() {
        let dir = temp_dir();
        let profile = AppProfile::custom(dir.join("cfg"), None);
        let mut store = Store::new(dir.join("store"), profile);
        let acc = store.add_account("Old".into(), None, None).unwrap();
        store
            .update_account(&acc.id, Some("New".into()), Some(Some("e@t.com".into())), None)
            .unwrap();
        let list = store.list_accounts();
        assert_eq!(list[0].name, "New");
    }

    #[test]
    fn test_switch_account_full_flow() {
        let dir = temp_dir();
        let cfg_dir = dir.join("cfg");
        let usr_dir = dir.join("usr");
        fs::create_dir_all(&cfg_dir).unwrap();
        fs::write(cfg_dir.join("settings.json"), r#"{"user":"default"}"#).unwrap();

        let profile = AppProfile::custom(cfg_dir.clone(), Some(usr_dir));
        let mut store = Store::new(dir.join("store"), profile);

        let acc1 = store.add_account("A".into(), None, None).unwrap();
        let acc2 = store.add_account("B".into(), None, None).unwrap();

        store.switch_account(&acc1.id).unwrap();
        assert_eq!(store.get_active_account().unwrap().name, "A");
        fs::write(cfg_dir.join("settings.json"), r#"{"user":"A"}"#).unwrap();

        store.switch_account(&acc2.id).unwrap();
        assert_eq!(store.get_active_account().unwrap().name, "B");

        store.switch_account(&acc1.id).unwrap();
        let content = fs::read_to_string(cfg_dir.join("settings.json")).unwrap();
        assert_eq!(content, r#"{"user":"A"}"#);
    }

    #[test]
    fn test_persistence() {
        let dir = temp_dir();
        let profile = AppProfile::custom(dir.join("cfg"), None);
        {
            let mut store = Store::new(dir.join("store"), profile.clone());
            store.add_account("P".into(), None, None).unwrap();
        }
        {
            let mut store = Store::new(dir.join("store"), profile);
            store.load().unwrap();
            assert_eq!(store.list_accounts().len(), 1);
            assert_eq!(store.list_accounts()[0].name, "P");
        }
    }
}
