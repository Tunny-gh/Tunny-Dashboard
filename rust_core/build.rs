fn main() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let libs_dir = manifest_dir.parent().unwrap().join("libs");
    // Link directly to the pre-built LightGBM shared library.
    // On Windows (MSVC): links lib_lightgbm.lib (import library generated from lib_lightgbm.dll).
    // On macOS: rustc-link-lib flags are emitted before rustc-link-search by the
    // linker driver, so ld never resolves the search path in time. The final
    // binary's build.rs (egui-app) handles macOS linking via rustc-link-arg instead.
    if !cfg!(target_os = "macos") {
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=lib_lightgbm");
    }

    // Copy lib_lightgbm.dll into the cargo target directory so that test
    // binaries (in target/<profile>/deps/) can find it at runtime on Windows.
    if cfg!(target_os = "windows") {
        let dll_src = libs_dir.join("lib_lightgbm.dll");
        if dll_src.exists() {
            // OUT_DIR = target/<profile>/build/<pkg>-<hash>/out/
            // Go up 3 levels to reach target/<profile>/
            let out_dir =
                std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
            if let Some(profile_dir) = out_dir.ancestors().nth(3) {
                let _ = std::fs::copy(&dll_src, profile_dir.join("lib_lightgbm.dll"));
                let deps_dir = profile_dir.join("deps");
                if deps_dir.exists() {
                    let _ = std::fs::copy(&dll_src, deps_dir.join("lib_lightgbm.dll"));
                }
            }
        }
    }

    println!("cargo:rerun-if-changed=build.rs");
}
