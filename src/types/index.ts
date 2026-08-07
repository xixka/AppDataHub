/**
 * AppDataHub 类型定义 — 严格对应 Rust 端数据结构
 */

/** 插件配置 (trae-cn.json 等) */
export interface PluginConfig {
  id: string;
  name: string;
  version: string;
  icon: string;
  homepage: string;
  process_names: string[];
  exe_candidates: string[];
  data_dirs: DataDirSpec[];
  skip_items: string[];
  machine_id: MachineIdSpec;
  login_artifacts: LoginArtifactSpec[];
  switch_flow: FlowStep[];
  clear_login_flow: FlowStep[];
}

export interface DataDirSpec {
  path: string; // 支持 %APPDATA% 等环境变量
  label: string;
  include_subdirs?: string[];
}

export interface MachineIdSpec {
  type: "file" | "registry";
  path: string; // 文件路径或注册表键路径
  key?: string; // 注册表值名
  label: string;
}

export interface LoginArtifactSpec {
  type: "file" | "dir" | "registry";
  path: string;
  key?: string;
}

/** 流程步骤 */
export type FlowStep =
  | { type: "ensure_not_running_or_kill"; timeout?: number }
  | { type: "backup_current" }
  | { type: "restore_snapshot" }
  | { type: "write_machine_id" }
  | { type: "reset_machine_id" }
  | { type: "delete_login_artifacts" }
  | { type: "launch_exe" }
  | { type: "sleep"; ms: number };

/** 账号 */
export interface Account {
  id: string;
  name: string;
  email: string | null;
  note: string | null;
  plugin_id: string;
  bound_machine_id: string | null;
  token_enc: string | null;
  created_at: string;
  last_used_at: string | null;
  is_active: boolean;
}

/** 账号元数据 (列表用) */
export interface AccountMetadata {
  id: string;
  name: string;
  email: string | null;
  note: string | null;
  plugin_id: string;
  has_snapshot: boolean;
  created_at: string;
  last_used_at: string | null;
  is_active: boolean;
}

/** 插件信息 */
export interface PluginInfo {
  id: string;
  name: string;
  version: string;
  icon: string;
  is_builtin: boolean;
  enabled: boolean;
  has_paths: boolean;
  exe_path: string | null;
}

/** Profile 信息 */
export interface ProfileInfo {
  config_dir: string;
  user_dir: string | null;
  exists: boolean;
}

/** 设置 */
export interface AppSettings {
  auto_kill: boolean;
  auto_launch_after_switch: boolean;
  theme: "light" | "dark";
}

/** 机器码信息 */
export interface MachineIdInfo {
  plugin_id: string;
  spec: MachineIdSpec;
  current_value: string | null;
  exists: boolean;
}

/** 日志条目 */
export interface LogEntry {
  timestamp: string;
  level: string;
  message: string;
}

/** API 错误 */
export class ApiError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ApiError";
  }
}
