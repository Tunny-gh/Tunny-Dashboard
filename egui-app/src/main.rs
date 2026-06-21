// Widget APIs are defined ahead of full UI wiring; suppress dead_code for now
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![allow(dead_code)]

mod app;
mod cli;
mod io;
mod licenses;
mod render;
mod state;
mod theme;
mod ui;

use cli::CliAction;

fn load_app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/TunnyIcon.png"))
        .expect("embedded app icon should be a valid PNG")
}

fn main() -> eframe::Result<()> {
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
