fn main() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let libs_dir = manifest_dir.parent().unwrap().join("libs");
    // Link directly to the pre-built LightGBM shared library.
    // On Windows (MSVC): links lib_lightgbm.lib (import library generated from lib_lightgbm.dll).
    if cfg!(target_os = "macos") {
        // On macOS, rustc-link-lib flags are placed *before* rustc-link-search by
        // the linker driver, so Apple ld never sees the search path in time and
        // reports "library not found". Emit -L and -l together via link-arg so
        // the search path always precedes the library. This applies to every
        // binary that links this crate, including `cargo test -p tunny-core`.
        // -rpath lets the produced binary find the dylib at runtime.
        println!("cargo:rustc-link-arg=-Wl,-L{}", libs_dir.display());
        println!("cargo:rustc-link-arg=-Wl,-l_lightgbm");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs_dir.display());
    } else if cfg!(target_os = "windows") {
        // Windows (MSVC): links lib_lightgbm.lib (import library from the DLL).
        // The literal file name is lib_lightgbm.lib, so the link name is `lib_lightgbm`.
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=lib_lightgbm");
    } else {
        // Linux: the shared object is `lib_lightgbm.so`. `rustc-link-lib=dylib=NAME`
        // links `libNAME.so`, so NAME must be `_lightgbm` (→ lib_lightgbm.so).
        // The previous `lib_lightgbm` wrongly resolved to `liblib_lightgbm.so`.
        println!("cargo:rustc-link-search=native={}", libs_dir.display());
        println!("cargo:rustc-link-lib=dylib=_lightgbm");
        // -rpath lets the produced binary/test locate the .so at runtime, matching
        // the macOS branch above.
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs_dir.display());
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
