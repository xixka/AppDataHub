/**
 * Tauri 命令封装 — 对应 Rust 端 #[tauri::command]
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  PluginConfig,
  PluginInfo,
  Account,
  AccountMetadata,
  ProfileInfo,
  AppSettings,
  MachineIdInfo,
  FlowResult,
} from "@/types";

// ===== 插件 =====
export const listPlugins = () => invoke<PluginInfo[]>("list_plugins");
export const reloadPlugins = () => invoke<void>("reload_plugins");
export const enablePlugin = (pluginId: string) =>
  invoke<void>("enable_plugin", { pluginId });
export const disablePlugin = (pluginId: string) =>
  invoke<void>("disable_plugin", { pluginId });
export const setPluginPaths = (params: { pluginId: string; exePath: string }) =>
  invoke<void>("set_plugin_paths", params);
export const getPluginConfig = (pluginId: string) =>
  invoke<PluginConfig>("get_plugin_config", { pluginId });

// ===== 账号 =====
export const listAccounts = (pluginId: string) =>
  invoke<AccountMetadata[]>("list_accounts", { pluginId });
export const addAccount = (params: {
  name: string;
  note: string | null;
  pluginId: string;
  machineId: string | null;
}) => invoke<Account>("add_account", params);
export const updateAccount = (params: {
  id: string;
  name: string;
  note: string | null;
}) => invoke<void>("update_account", params);
export const deleteAccount = (id: string) => invoke<void>("delete_account", { id });

// ===== 快照与切换 =====
export const saveSnapshot = (accountId: string) =>
  invoke<void>("save_snapshot", { accountId });
export const switchAccount = (accountId: string) =>
  invoke<FlowResult>("switch_account", { accountId });
export const clearLoginState = (pluginId: string) =>
  invoke<FlowResult>("clear_login_state", { pluginId });

// ===== 应用管理 =====
export const checkAppRunning = (pluginId: string) =>
  invoke<boolean>("check_app_running", { pluginId });
export const launchApp = (pluginId: string) =>
  invoke<void>("launch_app", { pluginId });

// ===== 机器码 =====
export const getMachineId = (pluginId: string) =>
  invoke<MachineIdInfo>("get_machine_id", { pluginId });
export const resetMachineId = (pluginId: string) =>
  invoke<void>("reset_machine_id", { pluginId });

// ===== 设置 =====
export const getSettings = () => invoke<AppSettings>("get_settings");
export const updateSettings = (settings: AppSettings) =>
  invoke<void>("update_settings", { settings });

// ===== 导入导出 =====
export const exportData = () => invoke<string>("export_data");
export const importData = (json: string) =>
  invoke<void>("import_data", { json });

// ===== 杂项 =====
export const openDataDir = () => invoke<void>("open_data_dir");
export const getLogsPath = () => invoke<string>("get_logs_path");
export const getLicense = () => invoke<string>("get_license");
