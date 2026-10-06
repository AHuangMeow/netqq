// Windows 上不弹控制台窗口
#![cfg_attr(windows, windows_subsystem = "windows")]

mod app;
mod http;
mod model;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("netqq")
            .with_inner_size([1000.0, 760.0])
            .with_min_inner_size([620.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(
        "netqq",
        options,
        Box::new(|cc| {
            theme::setup(&cc.egui_ctx);
            Ok(Box::new(app::App::new(cc)))
        }),
    )
}
