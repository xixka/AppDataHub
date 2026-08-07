//! Tauri 命令 — 前端可调用的接口

use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use tauri::State;

/// 启动可执行文件 (Windows 上通过 runas 触发 UAC 提权)
#[cfg(target_os = "windows")]
fn spawn_exe(exe_path: &str) -> Result<(), String> {
    let ps_cmd: &str = "Start-Process";
    let ps_path: &str = "-FilePath";
    let ps_verb: &str = "-Verb";
    let ps_verb_val: &str = "Runas";
    let flag_np: &str = "-NoProfile";
    let flag_cmd: &str = "-Command";
    let script = format!(
        "{} {} '{}' {} {}",
        ps_cmd, ps_path, exe_path.replace('\'', "''"), ps_verb, ps_verb_val
    );
    let output = std::process::Command::new("powershell")
        .arg(flag_np)
        .arg(flag_cmd)
        .arg(&script)
        .output()
        .map_err(|e| format!("启动失败: {}", e))?;
    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("OperationCanceled") || stderr.contains("0x800704C7") {
            Err("用户取消了提升权限".into())
        } else {
            Err(format!("启动失败: {}", stderr.trim()))
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn spawn_exe(exe_path: &str) -> Result<(), String> {
    std::process::Command::new(exe_path)
        .spawn()
        .map_err(|e| format!("启动失败: {}", e))?;
    Ok(())
}

use crate::account::{Account, AccountMetadata};
use crate::flow::{self, FlowContext, FlowResult, FlowSettings};
use crate::plugin::{PluginConfig, PluginError, PluginInfo};
use crate::store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Store(String),
    #[error("{0}")]
    Plugin(String),
    #[error("{0}")]
    Other(String),
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        CommandError::Store(e.to_string())
    }
}

impl From<PluginError> for CommandError {
    fn from(e: PluginError) -> Self {
        CommandError::Plugin(e.to_string())
    }
}

impl Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

// ===== 插件 =====

#[tauri::command]
pub fn list_plugins(
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<Vec<PluginInfo>, CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    Ok(mgr.list())
}

#[tauri::command]
pub fn reload_plugins(
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    mgr.reload(store.data_dir())?;
    Ok(())
}

#[tauri::command]
pub fn enable_plugin(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<(), CommandError> {
    let mut mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    mgr.enable_plugin(&plugin_id)?;
    Ok(())
}

#[tauri::command]
pub fn disable_plugin(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<(), CommandError> {
    let mut mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    mgr.disable_plugin(&plugin_id)?;
    Ok(())
}

#[tauri::command]
pub fn set_plugin_paths(
    plugin_id: String,
    exe_path: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<(), CommandError> {
    let mut mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    mgr.set_exe_path(&plugin_id, &exe_path)?;
    Ok(())
}

#[tauri::command]
pub fn get_plugin_config(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<PluginConfig, CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let cfg = mgr.get(&plugin_id)?;
    Ok(cfg.clone())
}

// ===== 账号 =====

#[tauri::command]
pub fn list_accounts(
    plugin_id: String,
    store: State<'_, Mutex<Store>>,
) -> Result<Vec<AccountMetadata>, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    Ok(store.list_accounts(&plugin_id))
}

#[tauri::command]
pub fn add_account(
    name: String,
    note: Option<String>,
    plugin_id: String,
    machine_id: Option<String>,
    store: State<'_, Mutex<Store>>,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<Account, CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?.clone();
    drop(mgr);

    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;

    let acc = store.add_account(name, note, plugin_id, machine_id, &plugin)?;
    Ok(acc)
}

#[tauri::command]
pub fn update_account(
    id: String,
    name: String,
    note: Option<String>,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    store.update_account(&id, name, note)?;
    Ok(())
}

#[tauri::command]
pub fn delete_account(
    id: String,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    store.delete_account(&id)?;
    Ok(())
}

#[tauri::command]
pub fn save_snapshot(
    account_id: String,
    store: State<'_, Mutex<Store>>,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;

    let acc = store.get_account(&account_id)?.clone();
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&acc.plugin_id)?.clone();

    drop(mgr);
    store.save_snapshot(&account_id, &plugin)?;
    Ok(())
}

#[tauri::command]
pub fn switch_account(
    account_id: String,
    store: State<'_, Mutex<Store>>,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<FlowResult, CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;

    let acc = store.get_account(&account_id)?.clone();
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&acc.plugin_id)?.clone();

    drop(mgr);
    let result = store.switch_account(&account_id, &plugin)?;
    Ok(result)
}

#[tauri::command]
pub fn clear_login_state(
    plugin_id: String,
    store: State<'_, Mutex<Store>>,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<FlowResult, CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?.clone();

    drop(mgr);
    let result = store.clear_login(&plugin)?;
    Ok(result)
}

// ===== 应用管理 =====

#[tauri::command]
pub fn check_app_running(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<bool, CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?;
    Ok(flow::is_app_running(&plugin))
}

#[tauri::command]
pub fn launch_app(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<(), CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?;
    let exe_path = mgr
        .get_exe_path(&plugin)
        .ok_or_else(|| CommandError::Plugin(format!("未找到 {} 可执行文件", plugin.name)))?;
    drop(mgr);

    spawn_exe(&exe_path).map_err(CommandError::Other)?;
    Ok(())
}

// ===== 机器码 =====

#[derive(Debug, Clone, Serialize)]
pub struct MachineIdInfo {
    pub plugin_id: String,
    pub spec: crate::plugin::MachineIdSpec,
    pub current_value: Option<String>,
    pub exists: bool,
}

#[tauri::command]
pub fn get_machine_id(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
) -> Result<MachineIdInfo, CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?;
    let spec = &plugin.machine_id;
    let value = flow::read_machine_id(spec)?;
    let exists = value.is_some();
    Ok(MachineIdInfo {
        plugin_id: plugin_id.clone(),
        spec: spec.clone(),
        current_value: value,
        exists,
    })
}

#[tauri::command]
pub fn reset_machine_id(
    plugin_id: String,
    plugin_mgr: State<'_, Mutex<crate::plugin::PluginManager>>,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mgr = plugin_mgr
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let plugin = mgr.get(&plugin_id)?;
    drop(mgr);

    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let dummy_account = Account {
        id: "__reset__".into(),
        name: "reset".into(),
        email: None,
        note: None,
        plugin_id: plugin_id.clone(),
        bound_machine_id: None,
        token_enc: None,
        created_at: chrono::Utc::now(),
        last_used_at: None,
        has_snapshot: false,
    };
    let snapshot_dir = store.snapshots_dir().join(&plugin_id).join("__reset__");
    let ctx = FlowContext {
        plugin,
        account: dummy_account,
        snapshot_dir,
        settings: Default::default(),
    };
    flow::reset_machine_id(&ctx)?;
    Ok(())
}

// ===== 设置 =====

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
}

impl From<&FlowSettings> for AppSettings {
    fn from(_s: &FlowSettings) -> Self {
        Self {}
    }
}

#[tauri::command]
pub fn get_settings(store: State<'_, Mutex<Store>>) -> Result<AppSettings, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    Ok(AppSettings::from(store.get_settings()))
}

#[tauri::command]
pub fn update_settings(
    _settings: AppSettings,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let flow_settings = FlowSettings {};
    store.update_settings(flow_settings)?;
    Ok(())
}

// ===== 导入导出 =====

#[tauri::command]
pub fn export_data(store: State<'_, Mutex<Store>>) -> Result<String, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    Ok(store.export_data()?)
}

#[tauri::command]
pub fn import_data(
    json: String,
    store: State<'_, Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    store.import_data(&json)?;
    Ok(())
}

// ===== 杂项 =====

#[tauri::command]
pub fn open_data_dir(store: State<'_, Mutex<Store>>) -> Result<(), CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    let path = store.data_dir().clone();
    open_folder(&path);
    Ok(())
}

#[tauri::command]
pub fn get_logs_path(store: State<'_, Mutex<Store>>) -> Result<String, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Other(e.to_string()))?;
    Ok(store.data_dir().join("logs").to_string_lossy().into_owned())
}

/// 获取内置 LICENSE 文本
#[tauri::command]
pub fn get_license() -> String {
    include_str!("../../LICENSE").to_string()
}

fn open_folder(path: &std::path::Path) {
    #[cfg(target_os = "windows")]
    {
        run_shell_cmd("explorer", &[path]);
    }
    #[cfg(target_os = "macos")]
    {
        run_shell_cmd("open", &[path]);
    }
    #[cfg(target_os = "linux")]
    {
        run_shell_cmd("xdg-open", &[path]);
    }
}

fn run_shell_cmd(cmd: &str, args: &[&std::path::Path]) {
    let _ = std::process::Command::new(cmd).args(args).spawn();
}
