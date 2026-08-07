//! WebView2Loader.dll 内嵌释放

use std::path::PathBuf;

pub fn setup() {
    let bytes: &[u8] = EMBEDDED_DLL;
    if bytes.is_empty() {
        return;
    }

    let temp_dir = get_temp_dir();
    let _ = std::fs::create_dir_all(&temp_dir);
    let dll_dest = temp_dir.join("WebView2Loader.dll");

    let need_write = match std::fs::metadata(&dll_dest) {
        Ok(meta) => meta.len() as usize != bytes.len(),
        Err(_) => true,
    };

    if need_write {
        let _ = std::fs::write(&dll_dest, bytes);
    }

    add_dll_directory(&temp_dir);
}

fn get_temp_dir() -> PathBuf {
    std::env::temp_dir().join("appdatahub")
}

#[cfg(target_os = "windows")]
fn add_dll_directory(dir: &std::path::Path) {
    use std::os::windows::ffi::OsStrExt;

    #[link(name = "kernel32")]
    extern "system" {
        fn SetDllDirectoryW(lpPathName: *const u16) -> i32;
    }

    let wide: Vec<u16> = dir.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        SetDllDirectoryW(wide.as_ptr());
    }
}

#[cfg(not(target_os = "windows"))]
fn add_dll_directory(_dir: &std::path::Path) {}

#[cfg(feature = "tauri-runtime")]
const EMBEDDED_DLL: &[u8] = include_bytes!(env!("WEBVIEW2_LOADER_DLL"));

#[cfg(not(feature = "tauri-runtime"))]
const EMBEDDED_DLL: &[u8] = &[];
