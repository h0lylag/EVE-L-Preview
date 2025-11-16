//! Hotkey settings component for profile configuration

use eframe::egui;
use crate::config::profile::Profile;
use crate::gui::constants::*;

/// State for hotkey settings UI
pub struct HotkeySettingsState {
    cycle_group_text: String,
}

impl HotkeySettingsState {
    pub fn new() -> Self {
        Self {
            cycle_group_text: String::new(),
        }
    }
    
    /// Load cycle group from profile into text buffer
    pub fn load_from_profile(&mut self, profile: &Profile) {
        self.cycle_group_text = profile.cycle_group.join("\n");
    }
    
    /// Parse text buffer back into profile's cycle group
    fn save_to_profile(&self, profile: &mut Profile) {
        profile.cycle_group = self.cycle_group_text
            .lines()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
    }
}

impl Default for HotkeySettingsState {
    fn default() -> Self {
        Self::new()
    }
}

/// Renders hotkey settings UI and returns true if changes were made
pub fn ui(ui: &mut egui::Ui, profile: &mut Profile, state: &mut HotkeySettingsState) -> bool {
    let mut changed = false;
    
    ui.group(|ui| {
        ui.label(egui::RichText::new("Character Cycle Order").strong());
        ui.add_space(ITEM_SPACING);
        
        ui.label("Enter character names (one per line, Tab/Shift+Tab to cycle):");
        
        ui.add_space(ITEM_SPACING / 2.0);
        
        // Multi-line text editor for cycle group
        let text_edit = egui::TextEdit::multiline(&mut state.cycle_group_text)
            .desired_rows(8)
            .desired_width(f32::INFINITY)
            .hint_text("Character Name 1\nCharacter Name 2\nCharacter Name 3");
        
        if ui.add(text_edit).changed() {
            // Update profile's cycle_group on every change
            state.save_to_profile(profile);
            changed = true;
        }
        
        ui.add_space(ITEM_SPACING / 2.0);
        
        ui.label(egui::RichText::new(
            format!("Current cycle order: {} character(s)", profile.cycle_group.len()))
            .small()
            .weak());
    });
    
    changed
}
