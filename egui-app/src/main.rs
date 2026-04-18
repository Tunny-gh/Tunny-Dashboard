// Widget APIs are defined ahead of full UI wiring; suppress dead_code for now
#![allow(dead_code)]

mod app;
mod io;
mod render;
mod state;
mod theme;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Tunny Dashboard",
        options,
        Box::new(|cc| Ok(Box::new(app::TunnyApp::new(cc)))),
    )
}
