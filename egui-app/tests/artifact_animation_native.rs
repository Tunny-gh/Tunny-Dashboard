//! Run with TUNNY_NATIVE_CAPTURE_CHECK=1. A custom harness keeps winit on the main thread.
#[path = "../src/lib.rs"]
// A main-thread harness embeds the crate so its private capture path is exercised.
// Ordinary #[test] functions are not run here; their fixture imports are unused.
#[allow(dead_code, unused_imports)]
mod desktop;
pub use desktop::*;

fn main() {
    if std::env::var_os("TUNNY_NATIVE_CAPTURE_CHECK").is_some() {
        ui::widgets::artifact_animation::native_capture_check();
    } else {
        println!("Native Artifact Animation capture check skipped (set TUNNY_NATIVE_CAPTURE_CHECK=1 on an interactive desktop).");
    }
}
