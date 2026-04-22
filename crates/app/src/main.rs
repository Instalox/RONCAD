//! Binary entry point. Initializes tracing, builds the eframe window,
//! and hands control to RonCadApp.

mod app;
mod bootstrap;
mod dispatcher;
mod interaction_controller;
mod settings;

use app::RonCadApp;

fn main() -> eframe::Result<()> {
    bootstrap::install_tracing();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("RONCAD")
            .with_inner_size([1400.0, 880.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "RONCAD",
        native_options,
        Box::new(|cc| Ok(Box::new(RonCadApp::new(cc)))),
    )
}
