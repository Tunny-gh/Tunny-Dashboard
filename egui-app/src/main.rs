mod app;
mod io;
mod render;
mod state;
mod ui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "Tunny Dashboard",
        options,
        Box::new(|cc| Ok(Box::new(app::TunnyApp::new(cc)))),
    )
}
