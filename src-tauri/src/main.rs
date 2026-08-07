// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use appdatahub_lib::{commands, loader_dll, plugin::PluginManager, store::Store};
use tauri::Manager;

fn main() {
    // 在 Windows 上安装 panic hook
    #[cfg(target_os = "windows")]
    {
        std::panic::set_hook(Box::new(|info: &std::panic::PanicHookInfo| {
            let msg = format!("AppDataHub 启动失败\n\n错误: {}\n\n请截图此错误并反馈。", info);
            show_message_box(&msg, "AppDataHub 错误");
        }));
    }

    // 释放内嵌的 WebView2Loader.dll
    loader_dll::setup();

    if let Err(e) = run_app() {
        let msg = format!("AppDataHub 启动失败\n\n{}", e);
        #[cfg(target_os = "windows")]
        show_message_box(&msg, "AppDataHub 错误");
        eprintln!("{}", msg);
    }
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 初始化日志
            let _ = tracing_subscriber::fmt::try_init();

            // 获取数据目录
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| {
                    dirs::data_dir()
                        .map(|d| d.join("appdatahub"))
                        .unwrap_or_else(|| std::path::PathBuf::from("./appdatahub-data"))
                });

            std::fs::create_dir_all(&data_dir)?;
            let data_dir = data_dir.canonicalize().unwrap_or(data_dir);

            // 初始化 store
            let mut store = Store::new(data_dir.clone());
            if let Err(e) = store.load() {
                eprintln!("警告: 加载账号数据失败: {}", e);
            }

            // 初始化插件管理器
            let plugins_dir = data_dir.join("plugins");
            std::fs::create_dir_all(&plugins_dir)?;
            let mut plugin_mgr = PluginManager::new(data_dir.clone(), plugins_dir);
            if let Err(e) = plugin_mgr.init_builtin() {
                eprintln!("警告: 初始化插件失败: {}", e);
            }

            app.manage(Mutex::new(store));
            app.manage(Mutex::new(plugin_mgr));

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 插件
            commands::list_plugins,
            commands::reload_plugins,
            commands::set_plugin_paths,
            commands::get_plugin_config,
            // 账号
            commands::list_accounts,
            commands::add_account,
            commands::update_account,
            commands::delete_account,
            commands::save_snapshot,
            commands::switch_account,
            commands::clear_login_state,
            // 应用管理
            commands::check_app_running,
            commands::launch_app,
            // 机器码
            commands::get_machine_id,
            commands::reset_machine_id,
            // 设置
            commands::get_settings,
            commands::update_settings,
            // 导入导出
            commands::export_data,
            commands::import_data,
            // 杂项
            commands::open_data_dir,
            commands::get_logs_path,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri 启动失败: {}", e).into())
}

#[cfg(target_os = "windows")]
fn show_message_box(message: &str, title: &str) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(
            hWnd: *const std::ffi::c_void,
            lpText: *const u16,
            lpCaption: *const u16,
            uType: u32,
        ) -> i32;
    }

    let text: Vec<u16> = std::ffi::OsStr::new(message)
        .encode_wide()
        .chain(Some(0))
        .collect();
    let caption: Vec<u16> = std::ffi::OsStr::new(title)
        .encode_wide()
        .chain(Some(0))
        .collect();

    unsafe {
        MessageBoxW(std::ptr::null(), text.as_ptr(), caption.as_ptr(), 0x10); // MB_ICONERROR
    }
}

#[cfg(not(target_os = "windows"))]
fn show_message_box(message: &str, _title: &str) {
    eprintln!("{}", message);
}
