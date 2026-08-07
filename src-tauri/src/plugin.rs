//! 插件配置 Schema — 对应 JSON 插件文件
//!
//! 所有插件均为内置编译时嵌入，不支持手动添加/删除。

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
    #[error("{0}")]
    Other(String),
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

/// 插件管理器 — 加载、查询内置插件
pub struct PluginManager {
    plugins_dir: PathBuf,
    exe_overrides: std::collections::HashMap<String, String>,
    disabled_plugins: Vec<String>,
}

impl PluginManager {
    pub fn new(plugins_dir: PathBuf) -> Self {
        Self {
            plugins_dir,
            exe_overrides: std::collections::HashMap::new(),
            disabled_plugins: Vec::new(),
        }
    }

    pub fn init_builtin(&mut self) -> Result<(), PluginError> {
        std::fs::create_dir_all(&self.plugins_dir)?;
        self.load_overrides()?;
        self.load_disabled()?;
        Ok(())
    }

    pub fn reload(&mut self, _data_dir: &std::path::Path) -> Result<(), PluginError> {
        self.load_overrides()?;
        self.load_disabled()?;
        Ok(())
    }

    fn load_overrides(&mut self) -> Result<(), PluginError> {
        let f = self.plugins_dir.join("exe-overrides.json");
        if f.exists() {
            self.exe_overrides = serde_json::from_str(&std::fs::read_to_string(f)?).unwrap_or_default();
        }
        Ok(())
    }

    fn load_disabled(&mut self) -> Result<(), PluginError> {
        let f = self.plugins_dir.join("disabled.json");
        if f.exists() {
            self.disabled_plugins = serde_json::from_str(&std::fs::read_to_string(f)?).unwrap_or_default();
        }
        Ok(())
    }

    fn save_disabled(&self) -> Result<(), PluginError> {
        std::fs::write(self.plugins_dir.join("disabled.json"), serde_json::to_string_pretty(&self.disabled_plugins)?)?;
        Ok(())
    }

    pub fn save_overrides(&self) -> Result<(), PluginError> {
        std::fs::write(self.plugins_dir.join("exe-overrides.json"), serde_json::to_string_pretty(&self.exe_overrides)?)?;
        Ok(())
    }

    pub fn set_exe_path(&mut self, plugin_id: &str, exe_path: &str) -> Result<(), PluginError> {
        self.exe_overrides.insert(plugin_id.to_string(), exe_path.to_string());
        self.save_overrides()?;
        Ok(())
    }

    pub fn enable_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        self.disabled_plugins.retain(|id| id != plugin_id);
        self.save_disabled()?;
        Ok(())
    }

    pub fn disable_plugin(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        if !self.disabled_plugins.contains(&plugin_id.to_string()) {
            self.disabled_plugins.push(plugin_id.to_string());
        }
        self.save_disabled()?;
        Ok(())
    }

    pub fn is_disabled(&self, plugin_id: &str) -> bool {
        self.disabled_plugins.contains(&plugin_id.to_string())
    }

    pub fn list(&self) -> Vec<PluginInfo> {
        let builtin_ids = vec!["trae-cn", "wukong", "autoclaw", "loomy"];
        let mut result = Vec::new();

        for id in &builtin_ids {
            if let Ok(cfg) = self.load_plugin(id) {
                let exe_path = self.get_exe_path(&cfg);
                result.push(PluginInfo {
                    id: cfg.id.clone(),
                    name: cfg.name,
                    version: cfg.version,
                    icon: cfg.icon,
                    is_builtin: true,
                    enabled: !self.is_disabled(&cfg.id),
                    has_paths: exe_path.is_some(),
                    exe_path,
                });
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
        // 所有插件均为内置
        let json = match id {
            "trae-cn" => builtin_trae_cn_json(),
            "wukong" => builtin_wukong_json(),
            "autoclaw" => builtin_autoclaw_json(),
            "loomy" => builtin_loomy_json(),
            _ => return Err(PluginError::NotFound(id.to_string())),
        };
        let cfg: PluginConfig = serde_json::from_str(json).map_err(PluginError::Serde)?;
        Ok(cfg)
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
    pub enabled: bool,
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

// ===== 内置插件 JSON =====

/// Trae CN — AI IDE (字节跳动)
fn builtin_trae_cn_json() -> &'static str {
    r#"{
  "id": "trae-cn",
  "name": "Trae CN",
  "version": "0.1.0",
  "icon": "🤖",
  "homepage": "https://www.trae.cn",
  "process_names": ["TRAE SOLO CN.exe", "TRAE SOLO CN"],
  "exe_candidates": [
    "C:/soft/TRAE SOLO CN/TRAE SOLO CN.exe"
  ],
  "data_dirs": [
    {
      "path": "%APPDATA%/TRAE SOLO CN/User",
      "label": "Trae用户数据",
      "include_subdirs": ["globalStorage"]
    },
    {
      "path": "%USERPROFILE%/.trae-cn",
      "label": "Trae用户扩展",
      "include_subdirs": ["attachments", "mcps", "memory", "plugins", "skills"]
    }
  ],
  "skip_items": ["Cache", "GPUCache", "Code Cache", "Service Worker", "logs", "History", "workspaceStorage", ".ckg", ".mcp_gallery_cache", "CachedData", "CachedProfilesData", "CachedConfigurations", "blob_storage", "Crashpad", "DawnGraphiteCache", "DawnWebGPUCache", "VMCache", "Shared Dictionary", "WebStorage", "ModularData", "monitor", "Partitions", "Backups", "Network", "Session Storage", "Local Storage", "IndexedDB"],
  "machine_id": {
    "type": "file",
    "path": "%APPDATA%/TRAE SOLO CN/machineid",
    "label": "Trae 机器码"
  },
  "login_artifacts": [
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/machineid" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/User/globalStorage/storage.json" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/User/globalStorage/state.vscdb" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/User/globalStorage/state.vscdb.backup" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/Local State" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/Preferences" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/Local Storage" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/Session Storage" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/IndexedDB" },
    { "type": "file", "path": "%APPDATA%/TRAE SOLO CN/Network/Cookies" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/Partitions/trae-webview/Local Storage" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/Partitions/trae-webview/Session Storage" },
    { "type": "dir", "path": "%APPDATA%/TRAE SOLO CN/Partitions/trae-webview/IndexedDB" }
  ],
  "switch_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "restore_snapshot" },
    { "type": "write_machine_id" }
  ],
  "clear_login_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "delete_login_artifacts" }
  ]
}"#
}

/// Wukong (悟空) — 钉钉 AI 办公助手
fn builtin_wukong_json() -> &'static str {
    r#"{
  "id": "wukong",
  "name": "悟空",
  "version": "0.1.0",
  "icon": "🐒",
  "homepage": "https://www.dingtalk.com",
  "process_names": ["DingTalkReal.exe", "DingTalk Real", "RealLauncher.exe"],
  "exe_candidates": [
    "C:/Program Files/Wukong/0.9.65-26061702/DingTalkReal.exe",
    "C:/Program Files/Wukong/RealLauncher.exe"
  ],
  "data_dirs": [
    {
      "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws",
      "label": "悟空DWS配置",
      "include_subdirs": []
    },
    {
      "path": "%USERPROFILE%/.real/users",
      "label": "悟空用户目录",
      "include_subdirs": []
    },
    {
      "path": "%APPDATA%/dingtalk-rewind-server/users",
      "label": "Rewind用户数据",
      "include_subdirs": []
    }
  ],
  "skip_items": ["Cache", "Code Cache", "DawnGraphiteCache", "DawnWebGPUCache", "GPUCache", "Logs", "blob_storage", "Crashpad", ".ckg", "apm-log-recovery", "alogs", "Shared Dictionary", "WebStorage", "SharedStorage", "MediaCache", "JumpListData", "dws.exe"],
  "machine_id": {
    "type": "file",
    "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws/identity.json",
    "label": "悟空设备标识"
  },
  "login_artifacts": [
    { "type": "file", "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws/token.json" },
    { "type": "file", "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws/identity.json" },
    { "type": "file", "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws/.data" },
    { "type": "file", "path": "%USERPROFILE%/.real/.bin/dws/bin/.dws/.data.lock" },
    { "type": "dir", "path": "%USERPROFILE%/.real/users" },
    { "type": "dir", "path": "%USERPROFILE%/.real/.cache" },
    { "type": "dir", "path": "%USERPROFILE%/.real/.config" },
    { "type": "dir", "path": "%USERPROFILE%/.real/.skill-providers" },
    { "type": "dir", "path": "%USERPROFILE%/.real/bootstrap" },
    { "type": "dir", "path": "%USERPROFILE%/.real/.browser-extension" },
    { "type": "file", "path": "%USERPROFILE%/.dws/settings.json" },
    { "type": "dir", "path": "%USERPROFILE%/.dws/plugins" },
    { "type": "dir", "path": "%APPDATA%/realdoc" },
    { "type": "dir", "path": "%APPDATA%/dingtalk-rewind-server/users" },
    { "type": "file", "path": "%APPDATA%/dingtalk-rewind-server/feature_flags.json" },
    { "type": "file", "path": "%APPDATA%/dingtalk-rewind-server/legacy-migration-status.v2.json" },
    { "type": "file", "path": "%APPDATA%/com.dingtalk.real/auth.json" },
    { "type": "file", "path": "%APPDATA%/com.dingtalk.real/token-cache.json" },
    { "type": "file", "path": "%APPDATA%/com.dingtalk.real/user-cache.json" },
    { "type": "dir", "path": "%APPDATA%/com.dingtalk.real/Local Storage" },
    { "type": "dir", "path": "%APPDATA%/com.dingtalk.real/Session Storage" },
    { "type": "dir", "path": "%APPDATA%/com.dingtalk.real/IndexedDB" },
    { "type": "dir", "path": "%LOCALAPPDATA%/com.dingtalk.real/EBWebView/Default/Local Storage" },
    { "type": "dir", "path": "%LOCALAPPDATA%/com.dingtalk.real/EBWebView/Default/Session Storage" },
    { "type": "dir", "path": "%LOCALAPPDATA%/com.dingtalk.real/EBWebView/Default/IndexedDB" }
  ],
  "switch_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "restore_snapshot" },
    { "type": "write_machine_id" }
  ],
  "clear_login_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "delete_login_artifacts" }
  ]
}"#
}

/// AutoClaw — AutoGLM 桌面客户端
fn builtin_autoclaw_json() -> &'static str {
    r#"{
  "id": "autoclaw",
  "name": "AutoClaw",
  "version": "0.1.0",
  "icon": "🦀",
  "homepage": "https://autoglm.aminer.cn",
  "process_names": ["AutoClaw.exe"],
  "exe_candidates": [
    "C:/Program Files/AutoClaw/AutoClaw.exe",
    "C:/Program Files (x86)/AutoClaw/AutoClaw.exe"
  ],
  "data_dirs": [
    {
      "path": "%APPDATA%/autoclaw",
      "label": "AutoClaw用户数据",
      "include_subdirs": []
    }
  ],
  "skip_items": ["Cache", "GPUCache", "Code Cache", "Service Worker", "logs", "CachedData", "CachedProfilesData", "CachedConfigurations", "blob_storage", "Crashpad", "DawnGraphiteCache", "DawnWebGPUCache", "VMCache", "Shared Dictionary", "WebStorage", "ModularData", "monitor", "Partitions", "Backups", "Network", "Session Storage", "Local Storage", "IndexedDB", "SharedStorage", "MediaCache", "JumpListData", "chrome-ext", "Dictionaries", "DIPS", "apm-log-recovery", "im", "launch.json", "log_sdk_v2.db", "official.json", "auth.json.backup", "settings.json.backup", "token-cache.json.backup", "user-cache.json.backup"],
  "machine_id": {
    "type": "file",
    "path": "%APPDATA%/autoclaw/identity/device.json",
    "label": "AutoClaw设备码"
  },
  "login_artifacts": [
    { "type": "file", "path": "%APPDATA%/autoclaw/auth.json" },
    { "type": "file", "path": "%APPDATA%/autoclaw/token-cache.json" },
    { "type": "file", "path": "%APPDATA%/autoclaw/user-cache.json" },
    { "type": "file", "path": "%APPDATA%/autoclaw/settings.json" },
    { "type": "file", "path": "%APPDATA%/autoclaw/identity/device.json" },
    { "type": "file", "path": "%APPDATA%/autoclaw/Local State" },
    { "type": "file", "path": "%APPDATA%/autoclaw/Preferences" },
    { "type": "dir", "path": "%APPDATA%/autoclaw/Local Storage" },
    { "type": "dir", "path": "%APPDATA%/autoclaw/Session Storage" },
    { "type": "dir", "path": "%APPDATA%/autoclaw/IndexedDB" },
    { "type": "dir", "path": "%APPDATA%/autoclaw/im" },
    { "type": "file", "path": "%APPDATA%/autoclaw/risk-agreement.json" }
  ],
  "switch_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "restore_snapshot" },
    { "type": "write_machine_id" }
  ],
  "clear_login_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "delete_login_artifacts" }
  ]
}"#
}

/// Loomy — AI 助手桌面客户端
fn builtin_loomy_json() -> &'static str {
    r#"{
  "id": "loomy",
  "name": "Loomy",
  "version": "0.1.0",
  "icon": "🌸",
  "homepage": "",
  "process_names": ["Loomy.exe"],
  "exe_candidates": [
    "C:/Program Files/Loomy/Loomy.exe"
  ],
  "data_dirs": [
    {
      "path": "%APPDATA%/loomy",
      "label": "Loomy用户数据",
      "include_subdirs": []
    }
  ],
  "skip_items": ["Cache", "GPUCache", "Code Cache", "Service Worker", "logs", "CachedData", "CachedProfilesData", "CachedConfigurations", "blob_storage", "Crashpad", "DawnGraphiteCache", "DawnWebGPUCache", "VMCache", "Shared Dictionary", "WebStorage", "ModularData", "monitor", "Partitions", "Backups", "Network", "Session Storage", "Local Storage", "IndexedDB", "SharedStorage", "MediaCache", "JumpListData", "Dictionaries", "DIPS", "loomy-installer-trace.log", "nexus-debug.log", "update-trace.log", "app-update-runtime.yml"],
  "machine_id": {
    "type": "file",
    "path": "%APPDATA%/loomy/app-ui-state.json",
    "label": "Loomy设备标识"
  },
  "login_artifacts": [
    { "type": "file", "path": "%APPDATA%/loomy/app-ui-state.json" },
    { "type": "file", "path": "%APPDATA%/loomy/pet-work.json" },
    { "type": "file", "path": "%APPDATA%/loomy/Local State" },
    { "type": "file", "path": "%APPDATA%/loomy/Preferences" },
    { "type": "dir", "path": "%APPDATA%/loomy/Local Storage" },
    { "type": "dir", "path": "%APPDATA%/loomy/Session Storage" },
    { "type": "dir", "path": "%APPDATA%/loomy/IndexedDB" },
    { "type": "dir", "path": "%APPDATA%/loomy/Partitions" }
  ],
  "switch_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "restore_snapshot" },
    { "type": "write_machine_id" }
  ],
  "clear_login_flow": [
    { "type": "ensure_not_running_or_kill", "timeout": 5000 },
    { "type": "delete_login_artifacts" }
  ]
}"#
}
