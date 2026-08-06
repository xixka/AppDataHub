//! 应用配置文件管理
//!
//! 本模块从 JSON 配置文件加载目标应用的路径信息。
//! 代码本身不硬编码任何应用路径——所有路径均由配置文件提供。
//!
//! 配置文件位置: {app_data_dir}/profiles.json
//! 配置文件格式:
//! ```json
//! [
//!   {
//!     "name": "示例应用",
//!     "config_dir": "%APPDATA%/MyApp",
//!     "user_dir": "%USERPROFILE%/.myapp",
//!     "process_names": ["myapp.exe"],
//!     "skip_items": ["Cache", "GPUCache"]
//!   }
//! ]
//! ```
//!
//! 环境变量 `%APPDATA%` `%USERPROFILE%` 等会在加载时自动展开。

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("无法定位应用配置目录")]
    NotFound,
    #[error("配置目录不存在: {0}")]
    DirNotFound(String),
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),
    #[error("路径错误: {0}")]
    Path(String),
    #[error("配置文件错误: {0}")]
    Parse(String),
}

impl From<serde_json::Error> for ConfigError {
    fn from(e: serde_json::Error) -> Self {
        ConfigError::Parse(e.to_string())
    }
}

/// 单个应用配置 (对应配置文件中一条目)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileConfig {
    /// 显示名
    pub name: String,
    /// 配置目录 (支持 %APPDATA% 等环境变量)
    pub config_dir: String,
    /// 用户数据目录 (可选, 支持 %USERPROFILE%)
    pub user_dir: Option<String>,
    /// 进程名 (用于检测是否运行)
    pub process_names: Vec<String>,
    /// 快照时跳过的目录/文件名
    pub skip_items: Vec<String>,
}

/// 运行时展开后的应用配置
#[derive(Debug, Clone, Serialize)]
pub struct AppProfile {
    pub name: String,
    pub config_dir: PathBuf,
    pub user_dir: Option<PathBuf>,
    pub process_names: Vec<String>,
    pub skip_items: Vec<String>,
}

impl AppProfile {
    /// 从配置文件加载所有 profile
    pub fn load_all(config_file: &Path) -> Result<Vec<ProfileConfig>, ConfigError> {
        if !config_file.exists() {
            return Ok(vec![]);
        }
        let content = fs::read_to_string(config_file)?;
        let profiles: Vec<ProfileConfig> = serde_json::from_str(&content)
            .map_err(|e| ConfigError::Parse(e.to_string()))?;
        Ok(profiles)
    }

    /// 保存 profiles 到配置文件
    pub fn save_all(
        config_file: &Path,
        profiles: &[ProfileConfig],
    ) -> Result<(), ConfigError> {
        let content = serde_json::to_string_pretty(profiles)?;
        if let Some(parent) = config_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(config_file, content)?;
        Ok(())
    }

    /// 从 ProfileConfig 展开环境变量, 创建运行时 profile
    pub fn from_config(cfg: &ProfileConfig) -> Result<Self, ConfigError> {
        Ok(Self {
            name: cfg.name.clone(),
            config_dir: expand_env(&cfg.config_dir),
            user_dir: cfg.user_dir.as_ref().map(|d| expand_env(d)),
            process_names: cfg.process_names.clone(),
            skip_items: cfg.skip_items.clone(),
        })
    }

    /// 从自定义路径创建 (用于测试)
    pub fn custom(config_dir: PathBuf, user_dir: Option<PathBuf>) -> Self {
        Self {
            name: "custom".into(),
            config_dir,
            user_dir,
            process_names: vec![],
            skip_items: default_skip_items(),
        }
    }

    pub fn exists(&self) -> bool {
        self.config_dir.exists()
    }

    /// 检测应用是否正在运行 (Windows: tasklist)
    pub fn is_running(&self) -> bool {
        if self.process_names.is_empty() {
            return false;
        }
        use std::process::Command;
        let output = Command::new("tasklist")
            .args(["/FO", "CSV", "/NH"])
            .output();
        match output {
            Ok(o) => {
                let text = String::from_utf8_lossy(&o.stdout).to_lowercase();
                self.process_names
                    .iter()
                    .any(|n| text.contains(&n.to_lowercase()))
            }
            Err(_) => false,
        }
    }

    /// 备份配置到目标目录
    pub fn backup_to(&self, dest: &Path) -> Result<(), ConfigError> {
        if !self.config_dir.exists() {
            return Err(ConfigError::DirNotFound(
                self.config_dir.to_string_lossy().into(),
            ));
        }
        copy_dir_filtered(&self.config_dir, dest, &self.skip_items)?;
        if let Some(ref user_dir) = self.user_dir {
            if user_dir.exists() {
                let user_dest = dest.join("_user_data");
                copy_dir_filtered(user_dir, &user_dest, &self.skip_items)?;
            }
        }
        Ok(())
    }

    /// 从快照恢复配置
    pub fn restore_from(&self, source: &Path) -> Result<(), ConfigError> {
        if !source.exists() {
            return Err(ConfigError::DirNotFound(
                source.to_string_lossy().into(),
            ));
        }
        if let Some(parent) = self.config_dir.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if self.config_dir.exists() {
            fs::remove_dir_all(&self.config_dir)?;
        }
        copy_dir_filtered(source, &self.config_dir, &[])?;
        let user_source = source.join("_user_data");
        if let Some(ref user_dir) = self.user_dir {
            if user_source.exists() {
                if user_dir.exists() {
                    let _ = fs::remove_dir_all(user_dir);
                }
                if let Some(parent) = user_dir.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                copy_dir_filtered(&user_source, user_dir, &[])?;
            }
        }
        Ok(())
    }
}

/// 展开环境变量 %APPDATA% %USERPROFILE% %LOCALAPPDATA% 等
fn expand_env(path: &str) -> PathBuf {
    let mut result = path.to_string();
    for (key, val) in &[
        ("%APPDATA%", std::env::var("APPDATA").unwrap_or_default()),
        ("%USERPROFILE%", std::env::var("USERPROFILE").unwrap_or_default()),
        ("%LOCALAPPDATA%", std::env::var("LOCALAPPDATA").unwrap_or_default()),
    ] {
        result = result.replace(key, val);
    }
    PathBuf::from(result)
}

fn default_skip_items() -> Vec<String> {
    vec![
        "Cache".into(),
        "Code Cache".into(),
        "GPUCache".into(),
        "ShaderCache".into(),
        "DawnCache".into(),
        "DawnGraphiteCache".into(),
        "Service Worker".into(),
        "blob_storage".into(),
        "IndexedDB".into(),
    ]
}

fn should_skip(name: &str, skip_items: &[String]) -> bool {
    skip_items.iter().any(|s| s.eq_ignore_ascii_case(name))
}

fn copy_dir_filtered(src: &Path, dst: &Path, skip_items: &[String]) -> Result<(), ConfigError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if should_skip(&name_str, skip_items) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_filtered(&src_path, &dst_path, skip_items)?;
        } else {
            if name_str.eq_ignore_ascii_case("LOCK") {
                continue;
            }
            let _ = fs::copy(&src_path, &dst_path);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_env() {
        std::env::set_var("APPDATA", "C:\\Users\\test\\AppData\\Roaming");
        let p = expand_env("%APPDATA%/MyApp");
        assert_eq!(p, PathBuf::from("C:\\Users\\test\\AppData\\Roaming/MyApp"));
    }

    #[test]
    fn test_should_skip() {
        let skip = default_skip_items();
        assert!(should_skip("Cache", &skip));
        assert!(should_skip("GPUCache", &skip));
        assert!(!should_skip("User", &skip));
    }

    #[test]
    fn test_load_save_profiles() {
        let tmp = std::env::temp_dir().join("appdatahub_test_cfg.json");
        let _ = fs::remove_file(&tmp);

        let profiles = vec![ProfileConfig {
            name: "TestApp".into(),
            config_dir: "%APPDATA%/TestApp".into(),
            user_dir: Some("%USERPROFILE%/.testapp".into()),
            process_names: vec!["testapp.exe".into()],
            skip_items: vec!["Cache".into()],
        }];

        AppProfile::save_all(&tmp, &profiles).unwrap();
        let loaded = AppProfile::load_all(&tmp).unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "TestApp");

        std::env::set_var("APPDATA", "C:\\Roaming");
        std::env::set_var("USERPROFILE", "C:\\Users\\test");
        let runtime = AppProfile::from_config(&loaded[0]).unwrap();
        assert_eq!(runtime.config_dir, PathBuf::from("C:\\Roaming/TestApp"));
        assert_eq!(runtime.user_dir, Some(PathBuf::from("C:\\Users\\test/.testapp")));

        let _ = fs::remove_file(&tmp);
    }

    #[test]
    fn test_backup_restore() {
        let tmp = std::env::temp_dir().join("appdatahub_test_br");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("source/User")).unwrap();
        fs::create_dir_all(tmp.join("source/Cache")).unwrap();
        fs::write(tmp.join("source/User/settings.json"), r#"{"k":"v"}"#).unwrap();
        fs::write(tmp.join("source/Cache/tmp.dat"), "cached").unwrap();

        let profile = AppProfile::custom(tmp.join("source"), None);
        let backup_dir = tmp.join("backup");
        profile.backup_to(&backup_dir).unwrap();
        assert!(backup_dir.join("User/settings.json").exists());
        assert!(!backup_dir.join("Cache").exists());

        let restore_dir = tmp.join("restore");
        fs::create_dir_all(&restore_dir).unwrap();
        let rp = AppProfile::custom(restore_dir.clone(), None);
        rp.restore_from(&backup_dir).unwrap();
        assert!(restore_dir.join("User/settings.json").exists());
    }
}
