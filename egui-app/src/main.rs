// Widget APIs are defined ahead of full UI wiring; suppress dead_code for now
#![allow(dead_code)]

mod app;
mod io;
mod render;
mod state;
mod theme;
mod ui;

fn load_app_icon() -> egui::IconData {
    eframe::icon_data::from_png_bytes(include_bytes!("../assets/TunnyIcon.png"))
        .expect("embedded app icon should be a valid PNG")
}

fn main() -> eframe::Result<()> {
    let initial_path: Option<std::path::PathBuf> = std::env::args().nth(1).map(Into::into);

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
