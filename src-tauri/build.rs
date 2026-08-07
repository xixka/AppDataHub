fn main() {
    #[cfg(feature = "tauri-runtime")]
    {
        // 设置 WEBVIEW2_LOADER_DLL 环境变量
        // CI 上可能没有 WebView2Loader.dll, 用空占位符
        let out_dir = std::env::var("OUT_DIR").unwrap_or_default();
        let placeholder = std::path::Path::new(&out_dir).join("empty_dll.bin");
        let _ = std::fs::write(&placeholder, b"");
        println!("cargo:rustc-env=WEBVIEW2_LOADER_DLL={}", placeholder.display());

        tauri_build::build()
    }
}
