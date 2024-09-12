mod app;

use app::FolderWatcherApp;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(eframe::egui::Vec2::new(600.0, 800.0)),
        ..Default::default()
    };

    eframe::run_native(
        "Folder Watcher",
        native_options,
        Box::new(|cc| Box::new(FolderWatcherApp::new(cc))),
    )
}
