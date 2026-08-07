//! 流程引擎 — 解析并执行插件定义的步骤
//!
//! 引擎是通用的: 不硬编码任何应用路径或逻辑
//! 所有具体操作由插件 JSON 中的 FlowStep 序列决定

use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::account::Account;
use crate::config::ResolvedDataDir;
use crate::plugin::{FlowStep, MachineIdSpec, PluginConfig, PluginError};

/// 执行上下文 — 包含当前操作所需的所有信息
pub struct FlowContext {
    pub plugin: PluginConfig,
    pub account: Account,
    pub snapshot_dir: PathBuf,
    pub settings: FlowSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FlowSettings {}

/// 执行结果
#[derive(Debug, Clone, Serialize)]
pub struct FlowResult {
    pub steps_executed: Vec<String>,
    pub success: bool,
    pub error: Option<String>,
}

impl FlowResult {
    fn ok(steps: Vec<String>) -> Self {
        Self {
            steps_executed: steps,
            success: true,
            error: None,
        }
    }
    fn failed(steps: Vec<String>, err: impl ToString) -> Self {
        Self {
            steps_executed: steps,
            success: false,
            error: Some(err.to_string()),
        }
    }
}

/// 执行流程
pub fn execute_flow(ctx: &FlowContext, steps: &[FlowStep]) -> FlowResult {
    let mut executed = Vec::new();

    for step in steps {
        let label = step_label(step);
        match execute_step(ctx, step) {
            Ok(()) => executed.push(label),
            Err(e) => return FlowResult::failed(executed, e),
        }
    }

    FlowResult::ok(executed)
}

fn step_label(step: &FlowStep) -> String {
    match step {
        FlowStep::EnsureNotRunningOrKill { timeout } => {
            format!("ensure_not_running_or_kill({}ms)", timeout)
        }
        FlowStep::BackupCurrent => "backup_current".into(),
        FlowStep::RestoreSnapshot => "restore_snapshot".into(),
        FlowStep::WriteMachineId => "write_machine_id".into(),
        FlowStep::ResetMachineId => "reset_machine_id".into(),
        FlowStep::RegenerateMachineId => "regenerate_machine_id".into(),
        FlowStep::DeleteLoginArtifacts => "delete_login_artifacts".into(),
        FlowStep::LaunchExe => "launch_exe".into(),
        FlowStep::Sleep { ms } => format!("sleep({}ms)", ms),
    }
}

fn execute_step(ctx: &FlowContext, step: &FlowStep) -> Result<(), PluginError> {
    match step {
        FlowStep::EnsureNotRunningOrKill { timeout } => {
            ensure_not_running_or_kill(ctx, *timeout)
        }
        FlowStep::BackupCurrent => backup_current(ctx),
        FlowStep::RestoreSnapshot => restore_snapshot(ctx),
        FlowStep::WriteMachineId => write_machine_id(ctx),
        FlowStep::ResetMachineId => reset_machine_id(ctx),
        FlowStep::RegenerateMachineId => {
            regenerate_machine_id(ctx).map(|_| ())
        }
        FlowStep::DeleteLoginArtifacts => delete_login_artifacts(ctx),
        FlowStep::LaunchExe => launch_exe(ctx),
        FlowStep::Sleep { ms } => {
            std::thread::sleep(Duration::from_millis(*ms));
            Ok(())
        }
    }
}

/// 检查应用是否运行, 如果运行则尝试 kill
fn ensure_not_running_or_kill(ctx: &FlowContext, timeout_ms: u64) -> Result<(), PluginError> {
    #[cfg(feature = "tauri-runtime")]
    {
        use sysinfo::{ProcessRefreshKind, RefreshKind, System};
        let mut sys = System::new();
        let refresh = RefreshKind::new().with_processes(ProcessRefreshKind::new());
        sys.refresh_specifics(refresh);

        let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);

        loop {
            let running = sys
                .processes()
                .values()
                .any(|p| {
                    let name = p.name().to_string_lossy().to_lowercase();
                    ctx.plugin
                        .process_names
                        .iter()
                        .any(|pn| name == pn.to_lowercase())
                });

            if !running {
                return Ok(());
            }

            // 直接 kill
            
            for p in sys.processes().values() {
                let name = p.name().to_string_lossy().to_lowercase();
                if ctx
                    .plugin
                    .process_names
                    .iter()
                    .any(|pn| name == pn.to_lowercase())
                {
                    let _ = p.kill();
                }
            }

            if std::time::Instant::now() > deadline {
                return Err(PluginError::FlowFailed(
                    "等待应用退出超时".into(),
                ));
            }

            std::thread::sleep(Duration::from_millis(500));
            sys.refresh_specifics(refresh);
        }
    }

    #[cfg(not(feature = "tauri-runtime"))]
    {
        let _ = (ctx, timeout_ms);
        Ok(())
    }
}

/// 备份当前配置到快照目录
fn backup_current(ctx: &FlowContext) -> Result<(), PluginError> {
    let target_dir = &ctx.snapshot_dir;
    // 清空旧快照
    if target_dir.exists() {
        let _ = fs::remove_dir_all(&target_dir);
    }
    fs::create_dir_all(&target_dir)?;

    for dir_spec in &ctx.plugin.data_dirs {
        let resolved = ResolvedDataDir::resolve(
            &dir_spec.path,
            &dir_spec.label,
            &dir_spec.include_subdirs,
        );
        let backup_name = resolved.label.replace(' ', "_");
        let dst = target_dir.join(&backup_name);
        resolved.backup_to(&dst, &ctx.plugin.skip_items)?;
    }

    Ok(())
}

/// 从快照恢复配置
fn restore_snapshot(ctx: &FlowContext) -> Result<(), PluginError> {
    let snapshot_dir = &ctx.snapshot_dir;
    if !snapshot_dir.exists() {
        return Err(PluginError::FlowFailed(format!(
            "账号 {} 没有快照数据",
            ctx.account.name
        )));
    }

    for dir_spec in &ctx.plugin.data_dirs {
        let resolved = ResolvedDataDir::resolve(
            &dir_spec.path,
            &dir_spec.label,
            &dir_spec.include_subdirs,
        );
        let backup_name = resolved.label.replace(' ', "_");
        let src = snapshot_dir.join(&backup_name);
        if src.exists() {
            resolved.restore_from(&src)?;
        }
    }

    Ok(())
}

/// 写入绑定的机器码
fn write_machine_id(ctx: &FlowContext) -> Result<(), PluginError> {
    let mid = &ctx.plugin.machine_id;
    let value = ctx
        .account
        .bound_machine_id
        .as_ref()
        .ok_or_else(|| PluginError::FlowFailed("账号未绑定机器码".into()))?;

    write_machine_id_value(mid, value)
}

/// 重置机器码 (删除文件/清除注册表)
pub fn reset_machine_id(ctx: &FlowContext) -> Result<(), PluginError> {
    let mid = &ctx.plugin.machine_id;
    let path = crate::plugin::expand_env(&mid.path);

    match mid.spec_type.as_str() {
        "file" => {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
            Ok(())
        }
        #[cfg(all(feature = "tauri-runtime", target_os = "windows"))]
        "registry" => {
            use winreg::enums::*;
            use winreg::RegKey;
            let hk = if path.starts_with("HKEY_CURRENT_USER") {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };
            let subpath = path.to_string_lossy();
            let subpath = subpath
                .trim_start_matches("HKEY_CURRENT_USER\\")
                .trim_start_matches("HKEY_LOCAL_MACHINE\\");
            let key = mid.key.as_deref().unwrap_or("");
            if let Ok(subkey) = RegKey::predef(hk).open_subkey(subpath) {
                let _ = subkey.delete_value(key);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// 生成全新随机机器码并写入 (file/registry), 同时同步 storage.json 的
/// telemetry.machineId / sqmId / devDeviceId —— 让目标应用认成全新设备,
/// 从而能重新触发"新设备"礼包/签到。参考 Trae-Account-Manager 的做法。
pub fn regenerate_machine_id(ctx: &FlowContext) -> Result<String, PluginError> {
    let mid = &ctx.plugin.machine_id;
    let new_id = uuid::Uuid::new_v4().to_string();

    // 1. 写入新机器码到文件 / 注册表
    write_machine_id_value(mid, &new_id)?;

    // 2. 同步 storage.json 的 telemetry 字段 (若存在)
    let _ = sync_trae_telemetry(ctx, &new_id);

    Ok(new_id)
}

/// 遍历插件 data_dirs 查找 storage.json, 更新其中的 telemetry.machineId
/// (= sha256(机器码)前32hex, 模拟 Trae 的 md5 风格)、telemetry.sqmId、
/// telemetry.devDeviceId 为全新随机值。
fn sync_trae_telemetry(ctx: &FlowContext, new_machine_id: &str) -> Result<(), PluginError> {
    use sha2::{Digest, Sha256};

    // telemetry.machineId: sha256(机器码) 前 32 hex (md5 风格)
    let mut hasher = Sha256::new();
    hasher.update(new_machine_id.as_bytes());
    let digest = hasher.finalize();
    let telemetry_id = hex::encode(&digest[..16]); // 16 字节 = 32 hex
    let new_sqm = format!("{{{}}}", uuid::Uuid::new_v4().to_string().to_uppercase());
    let new_dev = uuid::Uuid::new_v4().to_string();

    for dir_spec in &ctx.plugin.data_dirs {
        let resolved = ResolvedDataDir::resolve(
            &dir_spec.path,
            &dir_spec.label,
            &dir_spec.include_subdirs,
        );

        // 候选查找目录: include_subdirs 指定的子目录, 否则根目录
        let candidates: Vec<PathBuf> = if resolved.include_subdirs.is_empty() {
            vec![resolved.expanded.clone()]
        } else {
            resolved
                .include_subdirs
                .iter()
                .map(|s| resolved.expanded.join(s))
                .collect()
        };

        for base in candidates {
            let storage = base.join("storage.json");
            if !storage.exists() {
                continue;
            }
            // 读 -> 改 -> 写
            let content = match fs::read_to_string(&storage) {
                Ok(c) => c,
                Err(_) => continue,
            };
            let mut json: serde_json::Value = match serde_json::from_str(&content) {
                Ok(v) => v,
                Err(_) => continue,
            };
            let touched = if let Some(obj) = json.as_object_mut() {
                obj.insert(
                    "telemetry.machineId".into(),
                    serde_json::Value::String(telemetry_id.clone()),
                );
                obj.insert(
                    "telemetry.sqmId".into(),
                    serde_json::Value::String(new_sqm.clone()),
                );
                obj.insert(
                    "telemetry.devDeviceId".into(),
                    serde_json::Value::String(new_dev.clone()),
                );
                true
            } else {
                false
            };
            if touched {
                if let Ok(pretty) = serde_json::to_string_pretty(&json) {
                    let _ = fs::write(&storage, pretty);
                }
            }
        }
    }

    Ok(())
}

/// 删除登录痕迹
fn delete_login_artifacts(ctx: &FlowContext) -> Result<(), PluginError> {
    for artifact in &ctx.plugin.login_artifacts {
        let path = crate::plugin::expand_env(&artifact.path);
        match artifact.artifact_type.as_str() {
            "file" if path.exists() => {
                let _ = fs::remove_file(&path);
            }
            "dir" if path.exists() => {
                let _ = fs::remove_dir_all(&path);
            }
            _ => {}
        }
    }
    Ok(())
}

/// 启动应用
fn launch_exe(ctx: &FlowContext) -> Result<(), PluginError> {
    #[cfg(feature = "tauri-runtime")]
    {
        let exe_path = ctx
            .plugin
            .exe_candidates
            .first()
            .map(|p| crate::plugin::expand_env(p))
            .filter(|p| p.exists());

        if let Some(exe) = exe_path {
            std::process::Command::new(&exe)
                .spawn()
                .map_err(|e| PluginError::FlowFailed(format!("启动失败: {}", e)))?;
            Ok(())
        } else {
            Err(PluginError::NoExePath(ctx.plugin.id.clone()))
        }
    }

    #[cfg(not(feature = "tauri-runtime"))]
    {
        let _ = ctx;
        Ok(())
    }
}

/// 写入机器码值 (公共函数)
fn write_machine_id_value(spec: &MachineIdSpec, value: &str) -> Result<(), PluginError> {
    let path = crate::plugin::expand_env(&spec.path);

    match spec.spec_type.as_str() {
        "file" => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&path, value)?;
            Ok(())
        }
        #[cfg(all(feature = "tauri-runtime", target_os = "windows"))]
        "registry" => {
            use winreg::enums::*;
            use winreg::RegKey;
            let hk = if spec.path.starts_with("HKEY_CURRENT_USER") {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };
            let subpath = spec
                .path
                .trim_start_matches("HKEY_CURRENT_USER\\")
                .trim_start_matches("HKEY_LOCAL_MACHINE\\");
            let (key, _) = RegKey::predef(hk).create_subkey(subpath)?;
            let value_name = spec.key.as_deref().unwrap_or("");
            key.set_value(value_name, &value)?;
            Ok(())
        }
        _ => Ok(()),
    }
}

/// 读取当前机器码
pub fn read_machine_id(spec: &MachineIdSpec) -> Result<Option<String>, PluginError> {
    let path = crate::plugin::expand_env(&spec.path);

    match spec.spec_type.as_str() {
        "file" => {
            if path.exists() {
                let content = fs::read_to_string(&path)?;
                Ok(Some(content.trim().to_string()))
            } else {
                Ok(None)
            }
        }
        #[cfg(all(feature = "tauri-runtime", target_os = "windows"))]
        "registry" => {
            use winreg::enums::*;
            use winreg::RegKey;
            let hk = if spec.path.starts_with("HKEY_CURRENT_USER") {
                HKEY_CURRENT_USER
            } else {
                HKEY_LOCAL_MACHINE
            };
            let subpath = spec
                .path
                .trim_start_matches("HKEY_CURRENT_USER\\")
                .trim_start_matches("HKEY_LOCAL_MACHINE\\");
            if let Ok(key) = RegKey::predef(hk).open_subkey(subpath) {
                let value_name = spec.key.as_deref().unwrap_or("");
                if let Ok(val) = key.get_value::<String, _>(value_name) {
                    return Ok(Some(val));
                }
            }
            Ok(None)
        }
        _ => Ok(None),
    }
}

/// 判断应用是否正在运行
pub fn is_app_running(plugin: &PluginConfig) -> bool {
    #[cfg(feature = "tauri-runtime")]
    {
        use sysinfo::{ProcessRefreshKind, RefreshKind, System};
        let mut sys = System::new();
        sys.refresh_specifics(RefreshKind::new().with_processes(ProcessRefreshKind::new()));

        sys.processes().values().any(|p| {
            let name = p.name().to_string_lossy().to_lowercase();
            plugin
                .process_names
                .iter()
                .any(|pn| name == pn.to_lowercase())
        })
    }

    #[cfg(not(feature = "tauri-runtime"))]
    {
        let _ = plugin;
        false
    }
}
