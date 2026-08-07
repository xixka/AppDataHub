//! AppDataHub — AI 软件多账号切换管理器
//!
//! 架构: JSON 插件化流程引擎
//! Rust 后端是通用引擎，解析并执行 JSON 插件定义的步骤

pub mod account;
#[cfg(feature = "tauri-runtime")]
pub mod commands;
pub mod config;
pub mod crypto;
pub mod flow;
pub mod plugin;
pub mod store;

#[cfg(feature = "tauri-runtime")]
pub mod loader_dll;

pub use account::{Account, AccountMetadata};
pub use plugin::{PluginConfig, PluginManager};
pub use store::Store;
