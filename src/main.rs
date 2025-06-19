#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use app::FolderWatcherApp;
use eframe::egui;

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([600.0, 800.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Folder Watcher",
        native_options,
        Box::new(|cc| Ok(Box::new(FolderWatcherApp::new(cc)))),
    )
}
