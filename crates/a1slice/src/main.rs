mod app;
mod backend;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 760.0])
            .with_min_inner_size([420.0, 640.0])
            .with_title("A1 Slice"),
        ..Default::default()
    };
    eframe::run_native(
        "A1 Slice",
        options,
        Box::new(|cc| Ok(Box::new(app::A1App::new(cc)))),
    )
}
