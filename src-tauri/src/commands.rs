//! Tauri 命令 — 前端可调用的接口

use std::path::PathBuf;
use tauri::State;

use crate::account::AccountMetadata;
use crate::config::{self, ProfileConfig};
use crate::store::{Store, StoreError};

#[derive(Debug, thiserror::Error)]
pub enum CommandError {
    #[error("{0}")]
    Store(String),
    #[error("{0}")]
    Config(String),
}

impl From<StoreError> for CommandError {
    fn from(e: StoreError) -> Self {
        CommandError::Store(e.to_string())
    }
}

impl serde::Serialize for CommandError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

#[tauri::command]
pub fn list_accounts(
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<Vec<AccountMetadata>, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    Ok(store.list_accounts())
}

#[tauri::command]
pub fn add_account(
    name: String,
    email: Option<String>,
    note: Option<String>,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<String, CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    let account = store.add_account(name, email, note)?;
    Ok(account.id)
}

#[tauri::command]
pub fn delete_account(
    id: String,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.delete_account(&id)?;
    Ok(())
}

#[tauri::command]
pub fn update_account(
    id: String,
    name: Option<String>,
    email: Option<Option<String>>,
    note: Option<Option<String>>,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.update_account(&id, name, email, note)?;
    Ok(())
}

#[tauri::command]
pub fn switch_account(
    id: String,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.switch_account(&id)?;
    Ok(())
}

#[tauri::command]
pub fn save_current_snapshot(
    id: String,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.save_current_snapshot(&id)?;
    Ok(())
}

/// 获取当前 profile 信息
#[tauri::command]
pub fn get_profile_info(
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<ProfileInfo, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    let (cfg, usr, exists) = store.get_profile_info();
    Ok(ProfileInfo {
        config_dir: cfg.to_string_lossy().into_owned(),
        user_dir: usr.map(|p| p.to_string_lossy().into_owned()),
        exists,
    })
}

/// 手动设置 profile 路径
#[tauri::command]
pub fn set_profile_paths(
    config_dir: String,
    user_dir: Option<String>,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.set_profile_paths(PathBuf::from(config_dir), user_dir.map(PathBuf::from));
    Ok(())
}

/// 从配置文件加载所有可用 profiles
#[tauri::command]
pub fn list_profiles(
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<Vec<ProfileConfig>, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    Ok(store.list_profiles())
}

/// 切换当前使用的 profile
#[tauri::command]
pub fn select_profile(
    index: usize,
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<(), CommandError> {
    let mut store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    store.select_profile(index)?;
    Ok(())
}

#[tauri::command]
pub fn check_app_running(
    store: State<'_, std::sync::Mutex<Store>>,
) -> Result<bool, CommandError> {
    let store = store
        .lock()
        .map_err(|e| CommandError::Store(e.to_string()))?;
    Ok(store.is_app_running())
}

/// 自动检测已安装的应用 — 返回实际存在的 profiles
#[tauri::command]
pub fn detect_profile() -> Result<Vec<ProfileConfig>, CommandError> {
    Ok(config::detect_installed_profiles())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ProfileInfo {
    pub config_dir: String,
    pub user_dir: Option<String>,
    pub exists: bool,
}
