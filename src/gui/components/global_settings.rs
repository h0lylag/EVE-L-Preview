//! Global settings component (applies to all profiles)

use eframe::egui;
use crate::config::profile::GlobalSettings;
use crate::gui::constants::*;

/// Renders global settings UI and returns true if changes were made
pub fn ui(ui: &mut egui::Ui, global: &mut GlobalSettings) -> bool {
    let mut changed = false;
    
    ui.group(|ui| {
        ui.label(egui::RichText::new("Global Daemon Settings").strong());
        ui.add_space(ITEM_SPACING);
        
        ui.label(egui::RichText::new("These settings apply to all profiles")
            .italics()
            .weak());
        
        ui.add_space(ITEM_SPACING);
        
        // Log level
        ui.horizontal(|ui| {
            ui.label("Log Level:");
            
            egui::ComboBox::from_id_salt("log_level_combo")
                .selected_text(&global.log_level)
                .show_ui(ui, |ui| {
                    if ui.selectable_value(&mut global.log_level, "error".to_string(), "Error").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut global.log_level, "warn".to_string(), "Warn").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut global.log_level, "info".to_string(), "Info").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut global.log_level, "debug".to_string(), "Debug").changed() {
                        changed = true;
                    }
                    if ui.selectable_value(&mut global.log_level, "trace".to_string(), "Trace").changed() {
                        changed = true;
                    }
                });
        });
        
        ui.label(egui::RichText::new("Controls daemon logging verbosity (requires restart)")
            .small()
            .weak());
        
        ui.add_space(ITEM_SPACING);
        
        // Minimize clients on switch
        if ui.checkbox(&mut global.minimize_clients_on_switch, 
            "Minimize EVE clients when switching focus").changed() {
            changed = true;
        }
        
        ui.label(egui::RichText::new(
            "When clicking a thumbnail, minimize all other EVE clients")
            .small()
            .weak());
    });
    
    ui.add_space(SECTION_SPACING);
    
    // Behavior Settings (Global)
    ui.group(|ui| {
        ui.label(egui::RichText::new("Behavior Settings").strong());
        ui.add_space(ITEM_SPACING);
        
        // Hide when no focus
        if ui.checkbox(&mut global.hide_when_no_focus, 
            "Hide thumbnails when EVE loses focus").changed() {
            changed = true;
        }
        
        ui.label(egui::RichText::new(
            "When enabled, thumbnails disappear when no EVE window is focused")
            .small()
            .weak());
        
        ui.add_space(ITEM_SPACING);
        
        // Snap threshold
        ui.horizontal(|ui| {
            ui.label("Snap Threshold:");
            if ui.add(egui::Slider::new(&mut global.snap_threshold, 0..=50)
                .suffix(" px")).changed() {
                changed = true;
            }
        });
        
        ui.label(egui::RichText::new(
            "Distance for edge/corner snapping (0 = disabled)")
            .small()
            .weak());
    });
    
    ui.add_space(SECTION_SPACING);
    
    // Hotkey Settings (Global)
    ui.group(|ui| {
        ui.label(egui::RichText::new("Hotkey Settings").strong());
        ui.add_space(ITEM_SPACING);
        
        // Hotkey require EVE focus
        if ui.checkbox(&mut global.hotkey_require_eve_focus, 
            "Require EVE window focused for hotkeys to work").changed() {
            changed = true;
        }
        
        ui.label(egui::RichText::new(
            "When enabled, Tab/Shift+Tab only work when an EVE window is focused")
            .small()
            .weak());
        
        ui.add_space(ITEM_SPACING);
        ui.separator();
        ui.add_space(ITEM_SPACING);
        
        ui.label(egui::RichText::new("Custom Hotkey Editor").italics());
        ui.label("Future: Configure custom global hotkeys here");
        ui.label("• Screenshot hotkey");
        ui.label("• Quick minimize all");
        ui.label("• Toggle preview visibility");
    });
    
    changed
}
