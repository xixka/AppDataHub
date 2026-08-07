//! 插件配置 Schema — 对应 JSON 插件文件
//!
//! 插件文件位置: {app_data_dir}/plugins/{plugin_id}.json
//! 内置插件: 编译时 include_str! 嵌入

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),
    #[error("序列化错误: {0}")]
    Serde(#[from] serde_json::Error),
    #[error("配置错误: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("插件不存在: {0}")]
    NotFound(String),
    #[error("插件路径未配置: {0}")]
    NoExePath(String),
    #[error("流程执行失败: {0}")]
    FlowFailed(String),
}

/// 插件配置 (对应 JSON 文件)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub id: String,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub homepage: String,

    /// 进程名 (用于检测是否运行 / kill)
    pub process_names: Vec<String>,

    /// exe 候选路径 (环境变量展开, 用户可覆盖)
    pub exe_candidates: Vec<String>,

    /// 需要备份/恢复的数据目录
    pub data_dirs: Vec<DataDirSpec>,

    /// 快照时跳过的目录/文件名
    pub skip_items: Vec<String>,

    /// 机器码定义
    pub machine_id: MachineIdSpec,

    /// 登录痕迹 (清除登录状态时删除)
    pub login_artifacts: Vec<LoginArtifactSpec>,

    /// 切换账号流程步骤
    pub switch_flow: Vec<FlowStep>,

    /// 清除登录状态流程步骤
    pub clear_login_flow: Vec<FlowStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataDirSpec {
    /// 路径 (支持 %APPDATA% 等环境变量)
    pub path: String,
    /// 人类可读标签
    pub label: String,
    /// 只包含这些子目录 (可选, 为空则全部)
    #[serde(default)]
    pub include_subdirs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineIdSpec {
    /// "file" 或 "registry"
    #[serde(rename = "type")]
    pub spec_type: String,
    /// 文件路径或注册表键路径
    pub path: String,
    /// 注册表值名 (type=registry 时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    /// 人类可读标签
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginArtifactSpec {
    #[serde(rename = "type")]
    pub artifact_type: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
}

/// 流程步骤 — 使用 serde tag 区分类型
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum FlowStep {
    #[serde(rename = "ensure_not_running_or_kill")]
    EnsureNotRunningOrKill {
        #[serde(default = "default_timeout")]
        timeout: u64,
    },
    #[serde(rename = "backup_current")]
    BackupCurrent,
    #[serde(rename = "restore_snapshot")]
    RestoreSnapshot,
    #[serde(rename = "write_machine_id")]
    WriteMachineId,
    #[serde(rename = "reset_machine_id")]
    ResetMachineId,
    #[serde(rename = "delete_login_artifacts")]
    DeleteLoginArtifacts,
    #[serde(rename = "launch_exe")]
    LaunchExe,
    #[serde(rename = "sleep")]
    Sleep {
        ms: u64,
    },
}

fn default_timeout() -> u64 {
    5000
}

/// 插件管理器 — 加载、查询插件
pub struct PluginManager {
    plugins_dir: PathBuf,
    /// 用户路径覆盖 (plugin_id -> exe_path)
    exe_overrides: std::collections::HashMap<String, String>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            exe_overrides: std::collections::HashMap::new(),
        }
    }

    /// 初始化: 创建内置插件文件 (如果不存在)
    pub fn init_builtin(&mut self) -> Result<(), PluginError> {
        std::fs::create_dir_all(&self.plugins_dir)?;

        // Trae CN 内置插件
        let trae_path = self.plugins_dir.join("trae-cn.json");
        if !trae_path.exists() {
            std::fs::write(&trae_path, builtin_trae_cn_json())?;
        }

        self.load_overrides()?;
        Ok(())
    }

    /// 重新加载 (重读 overrides)
    pub fn reload(&mut self, _data_dir: &std::path::Path) -> Result<(), PluginError> {
        self.load_overrides()
    }

    fn load_overrides(&mut self) -> Result<(), PluginError> {
        let overrides_file = self.plugins_dir.join("exe-overrides.json");
        if overrides_file.exists() {
            let content = std::fs::read_to_string(&overrides_file)?;
            self.exe_overrides = serde_json::from_str(&content).unwrap_or_default();
        }
        Ok(())
    }

    pub fn save_overrides(&self) -> Result<(), PluginError> {
        let overrides_file = self.plugins_dir.join("exe-overrides.json");
        let content = serde_json::to_string_pretty(&self.exe_overrides)?;
        std::fs::write(&overrides_file, content)?;
        Ok(())
    }

    pub fn set_exe_path(&mut self, plugin_id: &str, exe_path: &str) -> Result<(), PluginError> {
        self.exe_overrides.insert(plugin_id.to_string(), exe_path.to_string());
        self.save_overrides()?;
        Ok(())
    }

    pub fn list(&self) -> Vec<PluginInfo> {
        let mut result = Vec::new();

        // 内置插件
        let builtin_ids = vec!["trae-cn"];
        for id in &builtin_ids {
            if let Ok(cfg) = self.load_plugin(id) {
                let exe_path = self.get_exe_path(&cfg);
                result.push(PluginInfo {
                    id: cfg.id,
                    name: cfg.name,
                    version: cfg.version,
                    icon: cfg.icon,
                    is_builtin: true,
                    has_paths: exe_path.is_some(),
                    exe_path,
                });
            }
        }

        // 用户插件 (plugins_dir 下的其他 json)
        if let Ok(entries) = std::fs::read_dir(&self.plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("json") {
                    continue;
                }
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem == "exe-overrides" || builtin_ids.contains(&stem) {
                    continue;
                }
                if let Ok(cfg) = self.load_plugin(stem) {
                    let exe_path = self.get_exe_path(&cfg);
                    result.push(PluginInfo {
                        id: cfg.id,
                        name: cfg.name,
                        version: cfg.version,
                        icon: cfg.icon,
                        is_builtin: false,
                        has_paths: exe_path.is_some(),
                        exe_path,
                    });
                }
            }
        }

        result
    }

    pub fn get(&self, plugin_id: &str) -> Result<PluginConfig, PluginError> {
        self.load_plugin(plugin_id)
    }

    pub fn get_exe_path(&self, cfg: &PluginConfig) -> Option<String> {
        // 优先用户覆盖
        if let Some(p) = self.exe_overrides.get(&cfg.id) {
            return Some(p.clone());
        }
        // 候选路径检测
        for candidate in &cfg.exe_candidates {
            let expanded = expand_env(candidate);
            if expanded.exists() {
                return Some(expanded.to_string_lossy().into_owned());
            }
        }
        None
    }

    fn load_plugin(&self, id: &str) -> Result<PluginConfig, PluginError> {
        // 先从文件加载
        let path = self.plugins_dir.join(format!("{}.json", id));
        if path.exists() {
            let content = std::fs::read_to_string(&path)?;
            let cfg: PluginConfig = serde_json::from_str(&content)?;
            return Ok(cfg);
        }
        // 内置
        match id {
            "trae-cn" => {
                let cfg: PluginConfig =
                    serde_json::from_str(builtin_trae_cn_json()).map_err(PluginError::Serde)?;
                Ok(cfg)
            }
            _ => Err(PluginError::NotFound(id.to_string())),
        }
    }
}

/// 展开后返回的插件信息 (给前端)
#[derive(Debug, Clone, Serialize)]
pub struct PluginInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub icon: String,
    pub is_builtin: bool,
    pub has_paths: bool,
    pub exe_path: Option<String>,
}

/// 展开环境变量 (%APPDATA% / %USERPROFILE% / %LOCALAPPDATA%)
pub fn expand_env(path: &str) -> PathBuf {
    let mut result = path.to_string();

    for (var, val) in [
        ("APPDATA", "APPDATA"),
        ("USERPROFILE", "USERPROFILE"),
        ("LOCALAPPDATA", "LOCALAPPDATA"),
        ("TEMP", "TEMP"),
        ("PROGRAMFILES", "PROGRAMFILES"),
        ("PROGRAMFILES(X86)", "PROGRAMFILES(X86)"),
    ] {
        if let Ok(env_val) = std::env::var(val) {
            result = result.replace(&format!("%{}%", var), &env_val);
        }
    }

    PathBuf::from(result)
}

/// 内置 Trae CN 插件 JSON
fn builtin_trae_cn_json() -> &'static str {
    r#"{
  "id": "trae-cn",
  "name": "Trae CN",
  "version": "0.1.0",
  "icon": "🤖",
  "homepage": "https://www.trae.cn",
  "process_names": ["Trae.exe", "Trae"],
  "exe_candidates": [
    "%LOCALAPPDATA%/Programs/Trae/Trae.exe",
    "%LOCALAPPDATA%/Trae/Trae.exe"
  ],
  "data_dirs": [
    {
      "path": "%APPDATA%/Trae",
      "label": "用户配置目录",
      "include_subdirs": ["User"]
    },
    {
      "path": "%USERPROFILE%/.trae",
      "label": "用户扩展目录",
      "include_subdirs": []
    }
  ],
  "skip_items": ["Cache", "GPUCache", "Code Cache", "Service Worker", "logs"],
  "machine_id": {
    "type": "file",
    "path": "%APPDATA%/Trae/Service Provider/trae__internal_services.machineid",
    "label": "Trae 机器码"
  },
  "login_artifacts": [
    {
      "type": "dir",
      "path": "%APPDATA%/Trae/Service Provider/trae__internal_services_auth_cache"
    },
    {
      "type": "dir",
      "path": "%APPDATA%/Trae/Service Provider/trae__internal_services_oidc"
    }
  ],
  "switch_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "backup_current" },
    { "type": "restore_snapshot" },
    { "type": "write_machine_id" }
  ],
  "clear_login_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "delete_login_artifacts" },
    { "type": "reset_machine_id" }
  ]
}"#
}
