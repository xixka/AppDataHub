//! 账号仓库 — 本地持久化存储
//!
//! 每个账号快照自包含:
//!   {app_data_dir}/snapshots/{plugin_id}/{account_name}/
//!     ├── account.json   ← 账号元数据
//!     └── <data_dir_label>/  ← 备份的数据目录

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

pub struct Store {
    data_dir: PathBuf,
    snapshots_dir: PathBuf,
    accounts: Vec<Account>,
    settings: FlowSettings,
}

impl Store {
    pub fn new(data_dir: PathBuf) -> Self {
        let snapshots_dir = data_dir.join("snapshots");
        Self {
            data_dir,
            snapshots_dir,
            accounts: Vec::new(),
            settings: FlowSettings::default(),
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

        // 兼容迁移：旧的 accounts.json → 各快照目录里的 account.json
        let old_accounts_file = self.data_dir.join("accounts.json");
        if old_accounts_file.exists() {
            self.migrate_old_accounts(&old_accounts_file)?;
            // 迁移完成后删除旧文件
            let _ = fs::remove_file(&old_accounts_file);
        }

        // 遍历 snapshots 目录加载账号
        self.accounts.clear();
        for plugin_entry in fs::read_dir(&self.snapshots_dir)? {
            let plugin_entry = plugin_entry?;
            if !plugin_entry.file_type()?.is_dir() {
                continue;
            }
            for account_entry in fs::read_dir(plugin_entry.path())? {
                let account_entry = account_entry?;
                if !account_entry.file_type()?.is_dir() {
                    continue;
                }
                let account_json = account_entry.path().join("account.json");
                if account_json.exists() {
                    match fs::read_to_string(&account_json) {
                        Ok(content) => {
                            if let Ok(mut acc) = serde_json::from_str::<Account>(&content) {
                                // 校验快照目录实际存在
                                acc.has_snapshot = snapshot_has_data(&account_entry.path());
                                self.accounts.push(acc);
                            }
                        }
                        Err(_) => continue,
                    }
                }
            }
        }

        Ok(())
    }

    /// 将旧的 accounts.json 迁移到各快照目录
    fn migrate_old_accounts(&self, old_file: &PathBuf) -> Result<(), StoreError> {
        let content = fs::read_to_string(old_file)?;
        let old_data: OldStoreData = serde_json::from_str(&content).unwrap_or_default();
        for acc in &old_data.accounts {
            let snapshot_dir = self.snapshots_dir.join(&acc.plugin_id).join(&acc.name);
            if snapshot_dir.exists() {
                let account_json = snapshot_dir.join("account.json");
                let json = serde_json::to_string_pretty(acc)?;
                fs::write(&account_json, json)?;
            }
        }
        Ok(())
    }

    /// 保存单个账号元数据到其快照目录
    fn save_account_meta(&self, account: &Account) -> Result<(), StoreError> {
        let snapshot_dir = self.snapshots_dir.join(&account.plugin_id).join(&account.name);
        fs::create_dir_all(&snapshot_dir)?;
        let account_json = snapshot_dir.join("account.json");
        let json = serde_json::to_string_pretty(account)?;
        fs::write(&account_json, json)?;
        Ok(())
    }

    pub fn list_accounts(&self, plugin_id: &str) -> Vec<AccountMetadata> {
        self.accounts
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
        // 检查重名（同插件下）
        if self
            .accounts
            .iter()
            .any(|a| a.plugin_id == plugin_id && a.name == name)
        {
            return Err(StoreError::DuplicateName(name));
        }

        let mut acc = Account::new(name, None, note, plugin_id.clone());
        acc.bound_machine_id = machine_id;

        // 创建快照目录并保存元数据
        let snapshot_dir = self.snapshots_dir.join(&acc.plugin_id).join(&acc.name);
        fs::create_dir_all(&snapshot_dir)?;
        self.save_account_meta(&acc)?;

        // 自动执行备份
        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc.clone(),
            snapshot_dir: snapshot_dir.clone(),
            settings: self.settings.clone(),
        };
        let backup_step = crate::plugin::FlowStep::BackupCurrent;
        let _ = flow::execute_flow(&ctx, std::slice::from_ref(&backup_step));
        acc.has_snapshot = snapshot_has_data(&snapshot_dir);

        // 更新保存 has_snapshot 后的元数据
        self.save_account_meta(&acc)?;
        self.accounts.push(acc.clone());

        Ok(acc)
    }

    pub fn get_account(&self, id: &str) -> Result<&Account, StoreError> {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or_else(|| StoreError::NotFound(id.to_string()))
    }

    pub fn get_account_mut(&mut self, id: &str) -> Result<&mut Account, StoreError> {
        self.accounts
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
        let old_name = self.get_account(id)?.name.clone();
        let plugin_id = self.get_account(id)?.plugin_id.clone();

        let acc = self.get_account_mut(id)?;
        acc.name = name.clone();
        acc.note = note;

        // 如果名字变了，迁移快照目录
        if old_name != name {
            let old_dir = self.snapshots_dir.join(&plugin_id).join(&old_name);
            let new_dir = self.snapshots_dir.join(&plugin_id).join(&name);
            if old_dir.exists() {
                if new_dir.exists() {
                    let _ = fs::remove_dir_all(&new_dir);
                }
                fs::rename(&old_dir, &new_dir)?;
            }
        }

        // 更新元数据
        let acc = self.get_account(id)?;
        let acc_clone = acc.clone();
        self.save_account_meta(&acc_clone)?;

        Ok(())
    }

    pub fn delete_account(&mut self, id: &str) -> Result<(), StoreError> {
        let acc = self.get_account(id)?;
        let plugin_id = acc.plugin_id.clone();
        let acc_name = acc.name.clone();
        // 删除整个快照目录（包括 account.json 和备份数据）
        let snapshot_dir = self.snapshots_dir.join(&plugin_id).join(&acc_name);
        if snapshot_dir.exists() {
            let _ = fs::remove_dir_all(&snapshot_dir);
        }
        self.accounts.retain(|a| a.id != id);
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
            snapshot_dir: snapshot_dir.clone(),
            settings: self.settings.clone(),
        };

        // 执行 backup_current 步骤
        let backup_step = crate::plugin::FlowStep::BackupCurrent;
        let _ = flow::execute_flow(&ctx, std::slice::from_ref(&backup_step));

        // 更新 has_snapshot 并重写 account.json
        let acc = self.get_account_mut(account_id)?;
        acc.has_snapshot = snapshot_has_data(&snapshot_dir);
        let acc_clone = acc.clone();
        self.save_account_meta(&acc_clone)?;

        Ok(())
    }

    pub fn switch_account(
        &mut self,
        account_id: &str,
        plugin: &PluginConfig,
    ) -> Result<crate::flow::FlowResult, StoreError> {
        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join(
            self.get_account(account_id)?.name.clone()
        );
        let acc = self.get_account_mut(account_id)?;
        acc.last_used_at = Some(Utc::now());
        let acc_clone = acc.clone();

        // 更新 last_used_at
        self.save_account_meta(&acc_clone)?;

        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: acc_clone,
            snapshot_dir,
            settings: self.settings.clone(),
        };

        let result = flow::execute_flow(&ctx, &plugin.switch_flow);
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
            has_snapshot: false,
        };

        let snapshot_dir = self.snapshots_dir.join(&plugin.id).join("__clear__");
        let ctx = FlowContext {
            plugin: plugin.clone(),
            account: dummy_account,
            snapshot_dir,
            settings: self.settings.clone(),
        };

        let result = flow::execute_flow(&ctx, &plugin.clear_login_flow);
        Ok(result)
    }

    pub fn get_settings(&self) -> &FlowSettings {
        &self.settings
    }

    pub fn update_settings(&mut self, settings: FlowSettings) -> Result<(), StoreError> {
        self.settings = settings;
        Ok(())
    }

    /// 导出所有数据为 JSON (accounts + 各快照元数据)
    pub fn export_data(&self) -> Result<String, StoreError> {
        #[derive(Serialize)]
        struct ExportData<'a> {
            accounts: &'a [Account],
        }
        let data = ExportData {
            accounts: &self.accounts,
        };
        Ok(serde_json::to_string_pretty(&data)?)
    }

    /// 导入 JSON 数据
    pub fn import_data(&mut self, json: &str) -> Result<(), StoreError> {
        #[derive(Deserialize)]
        struct ImportData {
            accounts: Vec<Account>,
        }
        let data: ImportData = serde_json::from_str(json)?;
        self.accounts = data.accounts;
        // 重写每个账号的 account.json
        for acc in &self.accounts {
            self.save_account_meta(acc)?;
        }
        Ok(())
    }
}

/// 检查快照目录中是否有实际备份数据（排除 account.json 本身）
fn snapshot_has_data(snapshot_dir: &std::path::Path) -> bool {
    if let Ok(entries) = fs::read_dir(snapshot_dir) {
        entries
            .filter_map(|e| e.ok())
            .any(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                name != "account.json" && name != "__clear__"
            })
    } else {
        false
    }
}

/// 旧的 StoreData 结构，仅用于迁移
#[derive(Debug, Clone, Deserialize, Default)]
struct OldStoreData {
    #[serde(default)]
    accounts: Vec<Account>,
}
