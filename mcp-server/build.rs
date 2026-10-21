fn main() {
    // rlib link arguments do not supply the final executable's runtime path.
    if cfg!(any(target_os = "macos", target_os = "linux")) {
        let libs = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("libs");
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", libs.display());
        if cfg!(target_os = "macos") {
            println!("cargo:rustc-link-arg=-Wl,-L{}", libs.display());
            println!("cargo:rustc-link-arg=-Wl,-l_lightgbm");
        }
    }
    println!("cargo:rerun-if-changed=build.rs");
}
