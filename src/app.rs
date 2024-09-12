use eframe::egui;
use rfd::FileDialog;
use serde::{Serialize, Deserialize};
use std::process::Command;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};

#[derive(Clone, Serialize, Deserialize)]
struct WatcherRow {
    path: String,
    commands: Vec<String>,
    is_watching: bool,
    #[serde(skip)]
    last_triggered: Option<Instant>,
    #[serde(skip)]
    file_count: usize,
}

impl Default for WatcherRow {
    fn default() -> Self {
        Self {
            path: String::new(),
            commands: vec!["notify-send 'Änderung bemerkt' 'Eine Änderung wurde festgestellt.'".to_string()],
            is_watching: false,
            last_triggered: None,
            file_count: 0,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct FolderWatcherApp {
    watcher_rows: Vec<WatcherRow>,
    all_watching: bool,
    #[serde(skip)]
    rx: Option<mpsc::Receiver<(usize, usize)>>,
    #[serde(skip)]
    stop_signal: Arc<Mutex<bool>>,
}

impl FolderWatcherApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = if let Ok(json) = fs::read_to_string("config.json") {
            serde_json::from_str(&json).unwrap_or_else(|_| Self::default())
        } else {
            Self::default()
        };
        
        // Sammle die Indizes der Zeilen, die als "watching" markiert sind
        let watching_indices: Vec<usize> = app.watcher_rows.iter()
            .enumerate()
            .filter(|(_, row)| row.is_watching)
            .map(|(index, _)| index)
            .collect();
        
        // Starte die Überwachung für alle gesammelten Indizes
        for &index in &watching_indices {
            app.start_watching(index);
        }
        
        app
    }
    fn save_config(&self) {
        let json = serde_json::to_string_pretty(self).unwrap();
        fs::write("config.json", json).expect("Unable to write config file");
    }

    fn add_new_row(&mut self) {
        self.watcher_rows.push(WatcherRow::default());
        self.save_config();
    }

    fn update_row(&mut self, row_index: usize, ui: &mut egui::Ui) -> bool {
        let mut row = self.watcher_rows[row_index].clone();
        let mut changed = false;
        let mut remove = false;

        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Pfad:");
                if ui.text_edit_singleline(&mut row.path).changed() {
                    changed = true;
                }
                if ui.button("Durchsuchen").clicked() {
                    if let Some(path) = FileDialog::new().pick_folder() {
                        row.path = path.display().to_string();
                        println!("Gewählter Pfad: {}", row.path);
                        changed = true;
                    }
                }
            });

            ui.horizontal(|ui| {
                ui.label("Befehle:");
                ui.vertical(|ui| {
                    let mut new_commands = Vec::new();
                    for command in &row.commands {
                        let mut command = command.clone();
                        ui.horizontal(|ui| {
                            if ui.text_edit_singleline(&mut command).changed() {
                                changed = true;
                            }
                            if ui.button("-").clicked() && row.commands.len() > 1 {
                                changed = true;
                            } else {
                                new_commands.push(command);
                            }
                        });
                    }
                    if changed {
                        row.commands = new_commands;
                    }
                    if ui.button("+").clicked() {
                        row.commands.push(String::new());
                        changed = true;
                    }
                });
            });

            ui.horizontal(|ui| {
                if ui.button(if row.is_watching { "Stop" } else { "Start" }).clicked() {
                    row.is_watching = !row.is_watching;
                    if row.is_watching {
                        println!("Überwachungsprozess für {} gestartet", row.path);
                        self.start_watching(row_index);
                    } else {
                        println!("Überwachungsprozess für {} gestoppt", row.path);
                        self.stop_watching(row_index);
                    }
                    changed = true;
                }

                if ui.button("Entfernen").clicked() {
                    remove = true;
                }

                ui.label(if row.is_watching { "Überwacht" } else { "Gestoppt" });

                if let Some(last_triggered) = row.last_triggered {
                    let elapsed = last_triggered.elapsed();
                    ui.label(format!("Zuletzt ausgelöst: vor {} Sekunden", elapsed.as_secs()));
                }

                ui.label(format!("Dateien im Ordner: {}", row.file_count));
            });
        });

        if changed {
            self.watcher_rows[row_index] = row;
            self.save_config();
        }

        remove
    }

    fn start_watching(&mut self, row_index: usize) {
        let row = &mut self.watcher_rows[row_index];
        let path = row.path.clone();
        let commands = row.commands.clone();
        
        let (tx, rx) = mpsc::channel();
        self.rx = Some(rx);

        let stop_signal = Arc::clone(&self.stop_signal);

        thread::spawn(move || {
            let mut last_count = count_files(&path);
            println!("Initiale Anzahl der Dateien in {}: {}", path, last_count);
            
            while !*stop_signal.lock().unwrap() {
                thread::sleep(Duration::from_secs(1));
                let current_count = count_files(&path);
                if current_count != last_count {
                    println!("Änderung in {} erkannt. Neue Anzahl: {}", path, current_count);
                    if let Err(e) = tx.send((row_index, current_count)) {
                        eprintln!("Fehler beim Senden der Änderung: {}", e);
                        break;
                    }
                    for command in &commands {
                        if let Err(e) = Command::new("sh")
                            .arg("-c")
                            .arg(command)
                            .spawn() {
                            eprintln!("Fehler beim Ausführen des Befehls: {}", e);
                        }
                    }
                    last_count = current_count;
                }
            }
        });
    }

    fn stop_watching(&mut self, row_index: usize) {
        let row = &mut self.watcher_rows[row_index];
        row.is_watching = false;
        *self.stop_signal.lock().unwrap() = true;
    }

    fn toggle_all_watchers(&mut self) {
        self.all_watching = !self.all_watching;
        for i in 0..self.watcher_rows.len() {
            let row = &mut self.watcher_rows[i];
            if self.all_watching && !row.is_watching {
                self.start_watching(i);
            } else if !self.all_watching && row.is_watching {
                self.stop_watching(i);
            }
        }
        self.save_config();
    }

    fn check_for_updates(&mut self) {
        if let Some(rx) = &self.rx {
            if let Ok((row_index, new_count)) = rx.try_recv() {
                if let Some(row) = self.watcher_rows.get_mut(row_index) {
                    row.file_count = new_count;
                    row.last_triggered = Some(Instant::now());
                }
            }
        }
    }
}

impl Default for FolderWatcherApp {
    fn default() -> Self {
        Self {
            watcher_rows: vec![WatcherRow::default()],
            all_watching: false,
            rx: None,
            stop_signal: Arc::new(Mutex::new(false)),
        }
    }
}

impl eframe::App for FolderWatcherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.check_for_updates();

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Ordner-Überwachung");
            
            if ui.button(if self.all_watching { "Alle stoppen" } else { "Alle starten" }).clicked() {
                self.toggle_all_watchers();
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                let mut i = 0;
                while i < self.watcher_rows.len() {
                    let remove = self.update_row(i, ui);
                    if remove {
                        self.watcher_rows.remove(i);
                        self.save_config();
                    } else {
                        i += 1;
                    }
                    ui.add_space(10.0);
                }
            });

            if ui.button("Neue Zeile hinzufügen").clicked() {
                self.add_new_row();
            }
        });

        ctx.request_repaint();
    }
}

fn count_files<P: AsRef<Path>>(path: P) -> usize {
    fs::read_dir(path)
        .map(|entries| entries.filter(|e| e.is_ok()).count())
        .unwrap_or(0)
}
