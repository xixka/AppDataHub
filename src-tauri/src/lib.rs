//! AppDataHub — Windows 应用数据切换工具

pub mod account;
pub mod config;
pub mod crypto;
pub mod loader_dll;
pub mod store;

#[cfg(feature = "tauri-runtime")]
pub mod commands;

pub use account::{Account, AccountMetadata};
pub use config::{AppProfile, ProfileConfig};
pub use store::Store;

#[cfg(feature = "tauri-runtime")]
pub use commands::*;
