// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;

use appdatahub_lib::{commands, config::AppProfile, loader_dll, store::Store};
use tauri::Manager;

fn main() {
    // 在 Windows 上安装 panic hook，崩溃时弹出消息框
    #[cfg(target_os = "windows")]
    {
        std::panic::set_hook(Box::new(|info: &std::panic::PanicHookInfo| {
            let msg = format!("AppDataHub 启动失败\n\n错误: {}\n\n请截图此错误并反馈。", info);
            show_message_box(&msg, "AppDataHub 错误");
        }));
    }

    // 在 Tauri 启动前释放内嵌的 WebView2Loader.dll
    loader_dll::setup();

    if let Err(e) = run_app() {
        let msg = format!("AppDataHub 启动失败\n\n{}", e);
        #[cfg(target_os = "windows")]
        show_message_box(&msg, "AppDataHub 错误");
        #[cfg(not(target_os = "windows"))]
        eprintln!("{}", msg);
    }
}

fn run_app() -> Result<(), Box<dyn std::error::Error>> {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| {
                    dirs::data_dir()
                        .map(|d| d.join("appdatahub"))
                        .unwrap_or_else(|| std::path::PathBuf::from("./appdatahub-data"))
                });

            let profiles_file = data_dir.join("profiles.json");
            let profile = if profiles_file.exists() {
                match AppProfile::load_all(&profiles_file) {
                    Ok(configs) if !configs.is_empty() => {
                        AppProfile::from_config(&configs[0]).unwrap_or_else(|_| {
                            AppProfile::custom(data_dir.join("default"), None)
                        })
                    }
                    _ => AppProfile::custom(data_dir.join("default"), None),
                }
            } else {
                AppProfile::custom(data_dir.join("default"), None)
            };

            let mut store = Store::new(data_dir, profile);
            if let Err(e) = store.load() {
                eprintln!("警告: 加载账号数据失败: {}", e);
            }

            app.manage(Mutex::new(store));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::add_account,
            commands::delete_account,
            commands::update_account,
            commands::switch_account,
            commands::save_current_snapshot,
            commands::get_profile_info,
            commands::set_profile_paths,
            commands::list_profiles,
            commands::select_profile,
            commands::check_app_running,
        ])
        .run(tauri::generate_context!())
        .map_err(|e| format!("Tauri 启动失败: {}", e).into())
}

#[cfg(target_os = "windows")]
fn show_message_box(message: &str, title: &str) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "user32")]
    extern "system" {
        fn MessageBoxW(hWnd: *const std::ffi::c_void, lpText: *const u16, lpCaption: *const u16, uType: u32) -> i32;
    }

    let text: Vec<u16> = std::ffi::OsStr::new(message).encode_wide().chain(Some(0)).collect();
    let caption: Vec<u16> = std::ffi::OsStr::new(title).encode_wide().chain(Some(0)).collect();

    unsafe {
        MessageBoxW(std::ptr::null(), text.as_ptr(), caption.as_ptr(), 0x10); // MB_ICONERROR
    }
}

#[cfg(not(target_os = "windows"))]
fn show_message_box(message: &str, _title: &str) {
    eprintln!("{}", message);
}
