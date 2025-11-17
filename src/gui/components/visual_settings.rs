use eframe::egui;
use crate::config::profile::Profile;
use crate::gui::constants::*;

pub fn ui(ui: &mut egui::Ui, profile: &mut Profile) -> bool {
    let mut changed = false;
    
    ui.group(|ui| {
        ui.label(egui::RichText::new("Visual Settings").strong());
        ui.add_space(ITEM_SPACING);
        
        // Opacity
        ui.horizontal(|ui| {
            ui.label("Opacity:");
            if ui.add(egui::Slider::new(&mut profile.opacity_percent, 0..=100)
                .suffix("%")).changed() {
                changed = true;
            }
        });
        
        // Border toggle
        ui.horizontal(|ui| {
            ui.label("Borders:");
            if ui.checkbox(&mut profile.border_enabled, "Enabled").changed() {
                changed = true;
            }
        });
        
        // Border settings (only if enabled)
        if profile.border_enabled {
            ui.indent("border_settings", |ui| {
                ui.horizontal(|ui| {
                    ui.label("Border Size:");
                    if ui.add(egui::DragValue::new(&mut profile.border_size)
                        .range(1..=20)).changed() {
                        changed = true;
                    }
                });
                
                ui.horizontal(|ui| {
                    ui.label("Border Color:");
                    if ui.text_edit_singleline(&mut profile.border_color).changed() {
                        changed = true;
                    }
                    
                    // Color picker button - parses hex string, shows picker, updates string
                    if let Ok(mut color) = parse_hex_color(&profile.border_color) {
                        if ui.color_edit_button_srgba(&mut color).changed() {
                            profile.border_color = format_hex_color(color);
                            changed = true;
                        }
                    }
                });
            });
        }
        
        ui.add_space(ITEM_SPACING);
        
        // Text settings
        ui.horizontal(|ui| {
            ui.label("Text Size:");
            if ui.add(egui::DragValue::new(&mut profile.text_size)
                .range(8..=48)).changed() {
                changed = true;
            }
        });
        
        ui.horizontal(|ui| {
            ui.label("Text Position:");
            ui.label("X:");
            if ui.add(egui::DragValue::new(&mut profile.text_x)
                .range(0..=100)).changed() {
                changed = true;
            }
            ui.label("Y:");
            if ui.add(egui::DragValue::new(&mut profile.text_y)
                .range(0..=100)).changed() {
                changed = true;
            }
        });
        
        ui.horizontal(|ui| {
            ui.label("Text Color:");
            if ui.text_edit_singleline(&mut profile.text_color).changed() {
                changed = true;
            }
            
            // Color picker button
            if let Ok(mut color) = parse_hex_color(&profile.text_color) {
                if ui.color_edit_button_srgba(&mut color).changed() {
                    profile.text_color = format_hex_color(color);
                    changed = true;
                }
            }
        });
    });
    
    changed
}

/// Parse hex color string - supports both #RRGGBB and #AARRGGBB formats
fn parse_hex_color(hex: &str) -> Result<egui::Color32, ()> {
    let hex = hex.trim_start_matches('#');
    
    match hex.len() {
        6 => {
            // RGB format - assume full opacity
            let rr = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?;
            let gg = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?;
            let bb = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?;
            Ok(egui::Color32::from_rgba_unmultiplied(rr, gg, bb, 255))
        }
        8 => {
            // ARGB format
            let aa = u8::from_str_radix(&hex[0..2], 16).map_err(|_| ())?;
            let rr = u8::from_str_radix(&hex[2..4], 16).map_err(|_| ())?;
            let gg = u8::from_str_radix(&hex[4..6], 16).map_err(|_| ())?;
            let bb = u8::from_str_radix(&hex[6..8], 16).map_err(|_| ())?;
            Ok(egui::Color32::from_rgba_unmultiplied(rr, gg, bb, aa))
        }
        _ => Err(()),
    }
}

/// Format egui Color32 to hex string (#AARRGGBB or #RRGGBB)
fn format_hex_color(color: egui::Color32) -> String {
    if color.a() == 255 {
        // Full opacity - use shorter RGB format
        format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
    } else {
        // Has transparency - use ARGB format
        format!("#{:02X}{:02X}{:02X}{:02X}", 
            color.a(), color.r(), color.g(), color.b())
    }
}
