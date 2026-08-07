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
pub struct FlowSettings {
    pub auto_kill: bool,
    pub auto_launch_after_switch: bool,
}

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

            if !ctx.settings.auto_kill {
                return Err(PluginError::FlowFailed(
                    "应用正在运行，请先关闭或启用自动结束".into(),
                ));
            }

            // 尝试 kill
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
    let target_dir = ctx.snapshot_dir.join(&ctx.account.id);
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
    let snapshot_dir = ctx.snapshot_dir.join(&ctx.account.id);
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
                HKCU
            } else {
                HKLM
            };
            let subpath = path
                .to_string_lossy()
                .trim_start_matches("HKEY_CURRENT_USER\\")
                .trim_start_matches("HKEY_LOCAL_MACHINE\\");
            let key = mid.key.as_deref().unwrap_or("");
            let _ = RegKey::predef(hk).delete_value(subpath, key);
            Ok(())
        }
        _ => Ok(()),
    }
}

/// 删除登录痕迹
fn delete_login_artifacts(ctx: &FlowContext) -> Result<(), PluginError> {
    for artifact in &ctx.plugin.login_artifacts {
        let path = crate::plugin::expand_env(&artifact.path);
        match artifact.artifact_type.as_str() {
            "file" => {
                if path.exists() {
                    let _ = fs::remove_file(&path);
                }
            }
            "dir" => {
                if path.exists() {
                    let _ = fs::remove_dir_all(&path);
                }
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
                HKCU
            } else {
                HKLM
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
                HKCU
            } else {
                HKLM
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
