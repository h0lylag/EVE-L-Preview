//! GUI manager - egui-based interface for controlling the preview daemon

use anyhow::{Context, Result};
use eframe::egui;
use std::process::{Child, Command};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

pub struct GuiManager {
    preview_process: Option<Child>,
    preview_running: Arc<AtomicBool>,
    config_path: String,
    status_message: String,
}

impl Default for GuiManager {
    fn default() -> Self {
        let config_path = dirs::config_dir()
            .map(|p| p.join("eve-l-preview/eve-l-preview.toml"))
            .and_then(|p| p.to_str().map(String::from))
            .unwrap_or_else(|| "~/.config/eve-l-preview/eve-l-preview.toml".to_string());
        
        Self {
            preview_process: None,
            preview_running: Arc::new(AtomicBool::new(false)),
            config_path,
            status_message: "Preview daemon not running".to_string(),
        }
    }
}

impl GuiManager {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
    
    fn start_preview(&mut self) {
        if self.preview_process.is_some() {
            self.status_message = "Preview already running!".to_string();
            return;
        }
        
        // Get the path to our own executable
        let exe_path = std::env::current_exe()
            .unwrap_or_else(|_| "eve-l-preview".into());
        
        match Command::new(exe_path)
            .arg("--preview")
            .spawn()
        {
            Ok(child) => {
                self.preview_process = Some(child);
                self.preview_running.store(true, Ordering::Relaxed);
                self.status_message = format!("Preview daemon started (PID: {})", 
                    self.preview_process.as_ref().unwrap().id());
            }
            Err(e) => {
                self.status_message = format!("Failed to start preview: {}", e);
            }
        }
    }
    
    fn stop_preview(&mut self) {
        if let Some(mut child) = self.preview_process.take() {
            match child.kill() {
                Ok(_) => {
                    let _ = child.wait(); // Clean up zombie process
                    self.preview_running.store(false, Ordering::Relaxed);
                    self.status_message = "Preview daemon stopped".to_string();
                }
                Err(e) => {
                    self.status_message = format!("Failed to stop preview: {}", e);
                }
            }
        } else {
            self.status_message = "Preview not running".to_string();
        }
    }
    
    fn check_preview_status(&mut self) {
        if let Some(child) = &mut self.preview_process {
            match child.try_wait() {
                Ok(Some(status)) => {
                    // Process exited
                    self.preview_process = None;
                    self.preview_running.store(false, Ordering::Relaxed);
                    self.status_message = format!("Preview daemon exited: {}", status);
                }
                Ok(None) => {
                    // Still running
                }
                Err(e) => {
                    self.status_message = format!("Error checking preview status: {}", e);
                }
            }
        }
    }
    
    fn open_config_editor(&self) {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "xdg-open".to_string());
        let _ = Command::new(editor)
            .arg(&self.config_path)
            .spawn();
    }
}

impl eframe::App for GuiManager {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Check if preview process is still alive
        self.check_preview_status();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("EVE-L Preview Manager");
            ui.add_space(10.0);
            
            // Status section
            ui.group(|ui| {
                ui.label(egui::RichText::new("Status").strong());
                ui.label(&self.status_message);
                
                if let Some(child) = &self.preview_process {
                    ui.label(format!("PID: {}", child.id()));
                }
            });
            
            ui.add_space(10.0);
            
            // Control buttons
            ui.horizontal(|ui| {
                if self.preview_process.is_none() {
                    if ui.button("▶ Start Preview Daemon").clicked() {
                        self.start_preview();
                    }
                } else {
                    if ui.button("⏹ Stop Preview Daemon").clicked() {
                        self.stop_preview();
                    }
                    
                    if ui.button("🔄 Restart Preview Daemon").clicked() {
                        self.stop_preview();
                        // Give it a moment to clean up
                        std::thread::sleep(std::time::Duration::from_millis(100));
                        self.start_preview();
                    }
                }
            });
            
            ui.add_space(20.0);
            
            // Configuration section
            ui.group(|ui| {
                ui.label(egui::RichText::new("Configuration").strong());
                ui.label(format!("Config file: {}", self.config_path));
                
                if ui.button("📝 Edit Config").clicked() {
                    self.open_config_editor();
                }
                
                ui.label(egui::RichText::new("Note: Restart preview daemon after editing config")
                    .small()
                    .italics());
            });
            
            ui.add_space(20.0);
            
            // Help section
            ui.group(|ui| {
                ui.label(egui::RichText::new("Quick Start").strong());
                ui.label("1. Click 'Start Preview Daemon' to show EVE window thumbnails");
                ui.label("2. Use Tab/Shift+Tab to cycle between EVE characters");
                ui.label("3. Right-click drag thumbnails to reposition them");
                ui.label("4. Left-click a thumbnail to focus that EVE window");
            });
        });
        
        // Request repaint to keep checking process status
        ctx.request_repaint_after(std::time::Duration::from_millis(500));
    }
    
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Clean up preview process on exit
        self.stop_preview();
    }
}

pub fn run_gui() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 500.0])
            .with_title("EVE-L Preview Manager"),
        ..Default::default()
    };
    
    eframe::run_native(
        "EVE-L Preview Manager",
        options,
        Box::new(|cc| Ok(Box::new(GuiManager::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run egui application: {}", e))?;
    
    Ok(())
}
