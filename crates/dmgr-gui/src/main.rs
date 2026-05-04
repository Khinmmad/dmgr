mod app;
mod audio;
mod worker;

fn main() -> eframe::Result<()> {
    env_logger::init();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("dmgr — Device Manager")
            .with_inner_size([1280.0, 760.0])
            .with_min_inner_size([900.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "dmgr",
        native_options,
        Box::new(|_cc| Ok(Box::new(app::DmgrApp::new()))),
    )
}
