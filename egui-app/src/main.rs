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

/// Sets the process's DPI awareness to "System Aware".
///
/// With winit's default (Per-Monitor Aware V2), whenever the window is moved between
/// displays with different resolutions (DPI), Windows issues `WM_DPICHANGED` and winit
/// applies the new window size each time, causing resizes to thrash continuously while
/// crossing a monitor boundary. Setting System Aware makes the process use a single DPI,
/// so `WM_DPICHANGED` no longer fires when crossing monitors (on a monitor with a
/// different DPI, the OS bitmap-scales the window, which looks slightly blurry).
///
/// DPI awareness must be set before winit configures it when constructing the
/// `EventLoop`, so this must be called first. Setting it beforehand makes winit's own
/// setting attempt fail and get ignored, so this setting takes priority.
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
