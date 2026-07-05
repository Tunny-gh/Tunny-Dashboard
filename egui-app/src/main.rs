#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod cli;
mod io;
mod licenses;
mod state;
mod theme;
mod ui;

use cli::CliAction;

fn load_app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/TunnyIcon.png"))
        .expect("embedded app icon should be a valid PNG")
}

/// プロセスの DPI awareness を「System Aware」に設定する。
///
/// winit のデフォルト（Per-Monitor Aware V2）では、解像度（DPI）の異なるディスプレイ間で
/// ウィンドウを移動するたびに Windows が `WM_DPICHANGED` を発行し、winit がその都度
/// ウィンドウサイズを適用するため、モニタ境界を跨ぐ間リサイズが連続して暴れる。
/// System Aware にするとプロセスは単一の DPI を使い、モニタ跨ぎで `WM_DPICHANGED` が
/// 発生しなくなる（別 DPI のモニタでは OS がビットマップ拡縮するためややぼやける）。
///
/// DPI awareness は winit が `EventLoop` 構築時に設定するため、その前に呼ぶ必要がある。
/// 先に設定しておけば winit 側の設定は失敗して無視され、こちらの指定が優先される。
#[cfg(windows)]
fn set_system_dpi_awareness() {
    use std::ffi::c_void;
    // DPI_AWARENESS_CONTEXT_SYSTEM_AWARE = (DPI_AWARENESS_CONTEXT)-2
    const DPI_AWARENESS_CONTEXT_SYSTEM_AWARE: isize = -2;
    #[link(name = "user32")]
    unsafe extern "system" {
        fn SetProcessDpiAwarenessContext(value: *mut c_void) -> i32;
    }
    unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE as *mut c_void);
    }
}

fn main() -> eframe::Result<()> {
    #[cfg(windows)]
    set_system_dpi_awareness();

    let initial_path = match cli::parse_args(std::env::args().skip(1)) {
        Ok(CliAction::Run { initial_path }) => initial_path,
        Ok(CliAction::PrintVersion) => {
            println!("{}", cli::version_text());
            return Ok(());
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    };

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Tunny Dashboard (Beta)")
            .with_inner_size([1280.0, 800.0])
            .with_icon(load_app_icon()),
        ..Default::default()
    };
    eframe::run_native(
        "Tunny Dashboard",
        options,
        Box::new(move |cc| Ok(Box::new(app::TunnyApp::new(cc, initial_path)))),
    )
}
