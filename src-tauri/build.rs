fn main() {
    #[cfg(feature = "tauri-runtime")]
    {
        if let Some(dll_path) = find_webview2_loader_dll() {
            println!("cargo:rustc-env=WEBVIEW2_LOADER_DLL={}", dll_path.display());
            println!("cargo:rustc-env=WEBVIEW2_DLL_EMBEDDED=1");
            println!("cargo:warning=WebView2Loader.dll embedded: {}", dll_path.display());
        } else {
            let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
            let placeholder = std::path::Path::new(&out_dir).join("empty_dll.bin");
            let _ = std::fs::write(&placeholder, b"");
            println!("cargo:rustc-env=WEBVIEW2_LOADER_DLL={}", placeholder.display());
            println!("cargo:warning=WebView2Loader.dll NOT found, using placeholder");
        }
        tauri_build::build()
    }
}

#[cfg(feature = "tauri-runtime")]
fn find_webview2_loader_dll() -> Option<std::path::PathBuf> {
    let mut search_paths = Vec::new();

    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        search_paths.push(std::path::PathBuf::from(out_dir));
    }
    if let Ok(cargo_home) = std::env::var("CARGO_HOME") {
        search_paths.push(std::path::PathBuf::from(cargo_home).join("registry/src"));
    }
    if let Ok(home) = std::env::var("HOME") {
        search_paths.push(std::path::PathBuf::from(home).join(".cargo/registry/src"));
    }
    if let Ok(up) = std::env::var("USERPROFILE") {
        search_paths.push(std::path::PathBuf::from(up).join(".cargo/registry/src"));
    }

    for base in &search_paths {
        if let Some(path) = search_dir_filtered(base, "x64") {
            return Some(path);
        }
    }
    // 回退: 任意非 arm64 的 DLL
    for base in &search_paths {
        if let Some(path) = search_dir_filtered(base, "") {
            return Some(path);
        }
    }
    None
}

#[cfg(feature = "tauri-runtime")]
fn search_dir_filtered(dir: &std::path::Path, prefer_arch: &str) -> Option<std::path::PathBuf> {
    if !dir.exists() {
        return None;
    }
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return None,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if let Some(found) = search_dir_filtered(&path, prefer_arch) {
                return Some(found);
            }
        } else if entry.file_name() == "WebView2Loader.dll" {
            let parent_name = path
                .parent()
                .and_then(|p| p.file_name())
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default();

            if !prefer_arch.is_empty() {
                if parent_name == prefer_arch {
                    return Some(path);
                }
            } else {
                if parent_name != "arm64" && parent_name != "arm" {
                    return Some(path);
                }
            }
        }
    }
    None
}
