mod app;
mod config;
mod editor;
mod i18n;
mod overlays;
mod session;
mod tree;
mod updater;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_app_id(app::APP_ID)
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([760.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Partner",
        options,
        Box::new(|cc| Ok(Box::new(app::PartnerApp::new(cc)))),
    )
}
