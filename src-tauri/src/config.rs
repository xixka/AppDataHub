//! 配置目录管理 + 备份/恢复工具
//!
//! 环境变量 %APPDATA% %USERPROFILE% 等会在加载时自动展开

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO 错误: {0}")]
    Io(#[from] io::Error),
    #[error("路径不存在: {0}")]
    NotFound(String),
    #[error("解析错误: {0}")]
    Parse(String),
}

/// 运行时展开后的数据目录
#[derive(Debug, Clone)]
pub struct ResolvedDataDir {
    pub raw_path: String,
    pub expanded: PathBuf,
    pub label: String,
    pub include_subdirs: Vec<String>,
}

impl ResolvedDataDir {
    pub fn resolve(raw_path: &str, label: &str, include_subdirs: &[String]) -> Self {
        let expanded = crate::plugin::expand_env(raw_path);
        Self {
            raw_path: raw_path.to_string(),
            expanded,
            label: label.to_string(),
            include_subdirs: include_subdirs.to_vec(),
        }
    }

    pub fn exists(&self) -> bool {
        self.expanded.exists()
    }

    /// 备份此目录到目标路径 (带 skip_items 过滤)
    pub fn backup_to(&self, dst: &Path, skip_items: &[String]) -> Result<(), ConfigError> {
        if !self.expanded.exists() {
            return Ok(());
        }

        if self.include_subdirs.is_empty() {
            copy_dir_filtered(&self.expanded, dst, skip_items)?;
        } else {
            for subdir in &self.include_subdirs {
                let src = self.expanded.join(subdir);
                if src.exists() {
                    let d = dst.join(subdir);
                    copy_dir_filtered(&src, &d, skip_items)?;
                }
            }
        }
        Ok(())
    }

    /// 从快照恢复此目录
    pub fn restore_from(&self, src: &Path) -> Result<(), ConfigError> {
        if !src.exists() {
            return Ok(());
        }

        // 清空当前目录内容 (如果存在)
        if self.expanded.exists() {
            // 只清空 include_subdirs 指定的子目录, 避免误删其他文件
            if self.include_subdirs.is_empty() {
                // 清空整个目录
                let _ = fs::remove_dir_all(&self.expanded);
                fs::create_dir_all(&self.expanded)?;
            } else {
                for subdir in &self.include_subdirs {
                    let d = self.expanded.join(subdir);
                    if d.exists() {
                        let _ = fs::remove_dir_all(&d);
                    }
                }
            }
        } else {
            fs::create_dir_all(&self.expanded)?;
        }

        // 复制快照
        if self.include_subdirs.is_empty() {
            copy_dir_all(src, &self.expanded)?;
        } else {
            for subdir in &self.include_subdirs {
                let s = src.join(subdir);
                if s.exists() {
                    let d = self.expanded.join(subdir);
                    fs::create_dir_all(&d)?;
                    copy_dir_all(&s, &d)?;
                }
            }
        }
        Ok(())
    }
}

/// 应该跳过的目录/文件名
pub fn should_skip(name: &str, skip_items: &[String]) -> bool {
    let name_lower = name.to_lowercase();
    skip_items.iter().any(|s| s.to_lowercase() == name_lower)
}

/// 递归复制目录 (带 skip_items 过滤)
pub fn copy_dir_filtered(src: &Path, dst: &Path, skip_items: &[String]) -> Result<(), ConfigError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if should_skip(&name, skip_items) {
            continue;
        }
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_filtered(&src_path, &dst_path, skip_items)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// 递归复制目录 (不过滤)
pub fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), ConfigError> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let src_path = entry.path();
        let dst_path = dst.join(&name);
        let ft = entry.file_type()?;
        if ft.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skip_items() {
        assert!(should_skip("Cache", &["Cache".into()]));
        assert!(should_skip("cache", &["Cache".into()]));
        assert!(!should_skip("User", &["Cache".into()]));
    }

    #[test]
    fn test_copy_dir_filtered() {
        let tmp = std::env::temp_dir().join("appdatahub_test_filtered");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("source/User")).unwrap();
        fs::create_dir_all(tmp.join("source/Cache")).unwrap();
        fs::write(tmp.join("source/User/settings.json"), r#"{"k":"v"}"#).unwrap();
        fs::write(tmp.join("source/Cache/tmp.dat"), "cached").unwrap();

        copy_dir_filtered(
            &tmp.join("source"),
            &tmp.join("backup"),
            &["Cache".into()],
        )
        .unwrap();
        assert!(tmp.join("backup/User/settings.json").exists());
        assert!(!tmp.join("backup/Cache").exists());

        let _ = fs::remove_dir_all(&tmp);
    }
}
