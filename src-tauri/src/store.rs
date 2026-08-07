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
        note: Option<String>,
        plugin_id: String,
        machine_id: Option<String>,
        plugin: &PluginConfig,
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

        let mut acc = Account::new(name, None, note, plugin_id.clone());

        // 如果有机器码定义, 绑定当前机器码
        acc.bound_machine_id = machine_id;

        self.data.accounts.push(acc.clone());

        // 自动保存当前快照
        let snapshot_dir = self.snapshots_dir.join(&plugin_id).join(&acc.name);
        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc.clone(),
            snapshot_dir,
            settings: self.data.settings.clone(),
        };
        let backup_step = crate::plugin::FlowStep::BackupCurrent;
        let _ = flow::execute_flow(&ctx, std::slice::from_ref(&backup_step));
        acc.has_snapshot = true;
        // 更新 pushed account 的 has_snapshot
        if let Some(a) = self.data.accounts.last_mut() {
            a.has_snapshot = true;
        }

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
        note: Option<String>,
    ) -> Result<(), StoreError> {
        // 先取旧名字用于迁移快照
        let old_name = self.get_account(id)?.name.clone();
        let plugin_id = self.get_account(id)?.plugin_id.clone();

        let acc = self.get_account_mut(id)?;
        let new_name = name.clone();
        acc.name = name;
        acc.note = note;

        // 如果名字变了，迁移快照目录
        if old_name != new_name {
            let old_dir = self.snapshots_dir.join(&plugin_id).join(&old_name);
            let new_dir = self.snapshots_dir.join(&plugin_id).join(&new_name);
            if old_dir.exists() {
                // 如果新目录已存在，先删除
                if new_dir.exists() {
                    let _ = fs::remove_dir_all(&new_dir);
                }
                fs::rename(&old_dir, &new_dir)?;
            }
        }

        self.save()?;
        Ok(())
    }

    pub fn delete_account(&mut self, id: &str) -> Result<(), StoreError> {
        let acc = self.get_account(id)?;
        let plugin_id = acc.plugin_id.clone();
        let acc_name = acc.name.clone();
        // 删除快照
        let snapshot_dir = self.snapshots_dir.join(&plugin_id).join(&acc_name);
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
        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join(&acc.name);

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
        // 1. 先备份当前活跃账号的快照
        let current_active = self.data.accounts.iter()
            .find(|a| a.plugin_id == plugin.id && a.is_active)
            .cloned();
        if let Some(cur) = &current_active {
            if cur.id != account_id {
                let cur_snapshot_dir = self.snapshots_dir.join(&plugin.id).join(&cur.name);
                let cur_ctx = FlowContext {
                    plugin: plugin.clone(),
                    account: cur.clone(),
                    snapshot_dir: cur_snapshot_dir,
                    settings: self.data.settings.clone(),
                };
                let backup_step = crate::plugin::FlowStep::BackupCurrent;
                let _ = flow::execute_flow(&cur_ctx, std::slice::from_ref(&backup_step));
                // 标记当前账号有快照
                if let Some(a) = self.data.accounts.iter_mut().find(|a| a.id == cur.id) {
                    a.has_snapshot = true;
                }
            }
        }

        // 2. 取消当前活跃
        for acc in &mut self.data.accounts {
            if acc.plugin_id == plugin.id {
                acc.is_active = false;
            }
        }

        // 3. 设置目标账号为活跃
        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join(
            self.get_account(account_id)?.name.clone()
        );
        let acc = self.get_account_mut(account_id)?;
        acc.is_active = true;
        acc.last_used_at = Some(Utc::now());
        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc.clone(),
            snapshot_dir,
            settings: self.data.settings.clone(),
        };

        // 4. 执行切换流程 (restore_snapshot + write_machine_id)
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

        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join("__clear__");
        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: dummy_account,
            snapshot_dir,
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
