//! 账号仓库 — 本地持久化存储
//!
//! 数据文件: {app_data_dir}/accounts.json
//! 配置快照: {app_data_dir}/snapshots/{plugin_id}/{account_id}/

use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::account::{Account, AccountMetadata};
use crate::flow::{self, FlowContext, FlowSettings};
use crate::plugin::{PluginConfig, PluginError};

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
    #[error("插件错误: {0}")]
    Plugin(#[from] PluginError),
    #[error("操作失败: {0}")]
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StoreData {
    pub accounts: Vec<Account>,
    pub settings: FlowSettings,
}

pub struct Store {
    data_dir: PathBuf,
    snapshots_dir: PathBuf,
    accounts_file: PathBuf,
    data: StoreData,
}

impl Store {
    pub fn new(data_dir: PathBuf) -> Self {
        let snapshots_dir = data_dir.join("snapshots");
        let accounts_file = data_dir.join("accounts.json");
        Self {
            data_dir,
            snapshots_dir,
            accounts_file,
            data: StoreData::default(),
        }
    }

    pub fn data_dir(&self) -> &PathBuf {
        &self.data_dir
    }

    pub fn snapshots_dir(&self) -> &PathBuf {
        &self.snapshots_dir
    }

    pub fn load(&mut self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.data_dir)?;
        fs::create_dir_all(&self.snapshots_dir)?;

        if self.accounts_file.exists() {
            let content = fs::read_to_string(&self.accounts_file)?;
            self.data = serde_json::from_str(&content).unwrap_or_default();
        } else {
            self.data = StoreData::default();
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> Result<(), StoreError> {
        let content = serde_json::to_string_pretty(&self.data)?;
        fs::write(&self.accounts_file, content)?;
        Ok(())
    }

    pub fn list_accounts(&self, plugin_id: &str) -> Vec<AccountMetadata> {
        self.data
            .accounts
            .iter()
            .filter(|a| a.plugin_id == plugin_id)
            .map(Into::into)
            .collect()
    }

    pub fn add_account(
        &mut self,
        name: String,
        email: Option<String>,
        note: Option<String>,
        plugin_id: String,
        machine_id: Option<String>,
    ) -> Result<Account, StoreError> {
        // 检查重名
        if self
            .data
            .accounts
            .iter()
            .any(|a| a.plugin_id == plugin_id && a.name == name)
        {
            return Err(StoreError::DuplicateName(name));
        }

        let mut acc = Account::new(name, email, note, plugin_id);

        // 如果有机器码定义, 绑定当前机器码
        acc.bound_machine_id = machine_id.map(|m| m);

        self.data.accounts.push(acc.clone());
        self.save()?;
        Ok(acc)
    }

    pub fn get_account(&self, id: &str) -> Result<&Account, StoreError> {
        self.data
            .accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    pub fn get_account_mut(&mut self, id: &str) -> Result<&mut Account, StoreError> {
        self.data
            .accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    pub fn update_account(
        &mut self,
        id: &str,
        name: String,
        email: Option<String>,
        note: Option<String>,
    ) -> Result<(), StoreError> {
        let acc = self.get_account_mut(id)?;
        acc.name = name;
        acc.email = email;
        acc.note = note;
        self.save()?;
        Ok(())
    }

    pub fn delete_account(&mut self, id: &str) -> Result<(), StoreError> {
        let acc = self.get_account(id)?;
        let plugin_id = acc.plugin_id.clone();
        // 删除快照
        let snapshot_dir = self.snapshots_dir.join(&plugin_id).join(id);
        if snapshot_dir.exists() {
            let _ = fs::remove_dir_all(&snapshot_dir);
        }
        self.data.accounts.retain(|a| a.id != id);
        self.save()?;
        Ok(())
    }

    pub fn save_snapshot(
        &mut self,
        account_id: &str,
        plugin: &PluginConfig,
    ) -> Result<(), StoreError> {
        let acc = self.get_account(account_id)?.clone();
        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join(account_id);

        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc,
            snapshot_dir,
            settings: self.data.settings.clone(),
        };

        // 执行 backup_current 步骤
        let backup_step = crate::plugin::FlowStep::BackupCurrent;
        let _ = flow::execute_flow(&ctx, std::slice::from_ref(&backup_step));

        let acc = self.get_account_mut(account_id)?;
        acc.has_snapshot = true;
        self.save()?;
        Ok(())
    }

    pub fn switch_account(
        &mut self,
        account_id: &str,
        plugin: &PluginConfig,
    ) -> Result<crate::flow::FlowResult, StoreError> {
        // 取消当前活跃
        for acc in &mut self.data.accounts {
            if acc.plugin_id == plugin.id {
                acc.is_active = false;
            }
        }

        let acc = self.get_account_mut(account_id)?;
        acc.is_active = true;
        acc.last_used_at = Some(Utc::now());

        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc.clone(),
            snapshot_dir: self.snapshots_dir.clone(),
            settings: self.data.settings.clone(),
        };

        let result = flow::execute_flow(&ctx, &plugin.switch_flow);

        self.save()?;
        Ok(result)
    }

    pub fn clear_login(
        &mut self,
        plugin: &PluginConfig,
    ) -> Result<crate::flow::FlowResult, StoreError> {
        let dummy_account = Account {
            id: "__clear__".into(),
            name: "clear".into(),
            email: None,
            note: None,
            plugin_id: plugin.id.clone(),
            bound_machine_id: None,
            token_enc: None,
            created_at: Utc::now(),
            last_used_at: None,
            is_active: false,
            has_snapshot: false,
        };

        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: dummy_account,
            snapshot_dir: self.snapshots_dir.clone(),
            settings: self.data.settings.clone(),
        };

        let result = flow::execute_flow(&ctx, &plugin.clear_login_flow);
        Ok(result)
    }

    pub fn get_settings(&self) -> &FlowSettings {
        &self.data.settings
    }

    pub fn update_settings(&mut self, settings: FlowSettings) -> Result<(), StoreError> {
        self.data.settings = settings;
        self.save()?;
        Ok(())
    }

    /// 导出所有数据为 JSON
    pub fn export_data(&self) -> Result<String, StoreError> {
        Ok(serde_json::to_string_pretty(&self.data)?)
    }

    /// 导入 JSON 数据
    pub fn import_data(&mut self, json: &str) -> Result<(), StoreError> {
        self.data = serde_json::from_str(json)?;
        self.save()?;
        Ok(())
    }
}
