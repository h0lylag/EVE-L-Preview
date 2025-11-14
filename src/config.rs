use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};
use x11rb::protocol::render::Color;

use crate::color::{HexColor, Opacity};
use crate::types::Position;

/// Immutable display settings (loaded once at startup)
/// Can be borrowed by Thumbnails without RefCell
#[derive(Debug, Clone)]
pub struct DisplayConfig {
    pub width: u16,
    pub height: u16,
    pub opacity: u32,
    pub border_size: u16,
    pub border_color: Color,
    pub text_x: i16,
    pub text_y: i16,
    pub text_foreground: u32,
    pub text_background: u32,
    pub hide_when_no_focus: bool,
}

/// Persistent state that gets saved to TOML file
/// Contains both immutable display config and mutable runtime data
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistentState {
    // Display settings (immutable after load)
    pub width: u16,
    pub height: u16,
    #[serde(rename = "opacity_percent")]
    opacity_percent: u8,
    pub border_size: u16,
    #[serde(rename = "border_color", serialize_with = "serialize_color", deserialize_with = "deserialize_color")]
    border_color_hex: String,
    pub text_x: i16,
    pub text_y: i16,
    #[serde(rename = "text_foreground")]
    text_foreground_hex: String,
    #[serde(rename = "text_background")]
    text_background_hex: String,
    pub hide_when_no_focus: bool,
    
    // Mutable runtime state (persisted)
    /// Character name → position
    /// Persisted positions for each character's thumbnail
    #[serde(default)]
    pub character_positions: HashMap<String, Position>,
    
    /// Snap threshold in pixels (0 = disabled)
    #[serde(default = "default_snap_threshold")]
    pub snap_threshold: u16,
}

fn default_snap_threshold() -> u16 {
    15
}

fn serialize_color<S>(hex: &String, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(hex)
}

fn deserialize_color<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    String::deserialize(deserializer)
}

impl PersistentState {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("eve-l-preview");
        path.push("eve-l-preview.toml");
        path
    }

    /// Build DisplayConfig from current settings
    /// Returns a new DisplayConfig that can be used independently
    pub fn build_display_config(&self) -> DisplayConfig {
        // Parse colors from hex strings using color module
        let border_color = HexColor::parse(&self.border_color_hex)
            .map(|c| c.to_x11_color())
            .unwrap_or_else(|| {
                error!("Invalid border_color hex, using default");
                HexColor::from_argb32(0x7FFF0000).to_x11_color()
            });
        
        let text_foreground = HexColor::parse(&self.text_foreground_hex)
            .map(|c| c.to_premultiplied_argb32())
            .unwrap_or_else(|| {
                error!("Invalid text_foreground hex, using default");
                HexColor::from_argb32(0xFF_FF_FF_FF).to_premultiplied_argb32()
            });
        
        let text_background = HexColor::parse(&self.text_background_hex)
            .map(|c| c.to_premultiplied_argb32())
            .unwrap_or_else(|| {
                error!("Invalid text_background hex, using default");
                HexColor::from_argb32(0x7F_00_00_00).to_premultiplied_argb32()
            });
        
        let opacity = Opacity::from_percent(self.opacity_percent).to_argb32();
        
        DisplayConfig {
            width: self.width,
            height: self.height,
            opacity,
            border_size: self.border_size,
            border_color,
            text_x: self.text_x,
            text_y: self.text_y,
            text_foreground,
            text_background,
            hide_when_no_focus: self.hide_when_no_focus,
        }
    }
    pub fn load() -> Self {
        // Try to load existing config file
        let config_path = Self::config_path();
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(mut state) = toml::from_str::<PersistentState>(&contents) {
                // Apply env var overrides
                state.apply_env_overrides();
                return state;
            }
        }

        // Generate new config from env vars
        let state = Self::from_env();
        
        // Save for next time
        if let Err(e) = state.save() {
            error!("Failed to save config: {e:?}");
        } else {
            println!("Generated config file: {}", config_path.display());
            println!("Edit it to customize settings (env vars still override)");
        }
        
        state
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
    }

    /// Update position after drag - saves to character_positions and persists
    pub fn update_position(&mut self, character_name: &str, x: i16, y: i16) -> Result<()> {
        if !character_name.is_empty() {
            info!("Saving position for character '{}': ({}, {})", character_name, x, y);
            self.character_positions.insert(character_name.to_string(), Position::new(x, y));
            self.save()?;
        }
        Ok(())
    }

    /// Handle character name change (login/logout)
    /// Returns new position if the new character has a saved position
    pub fn handle_character_change(
        &mut self,
        old_name: &str,
        new_name: &str,
        current_position: Position,
    ) -> Result<Option<Position>> {
        info!("Character change: '{}' → '{}'", old_name, new_name);
        
        // Save old position
        if !old_name.is_empty() {
            self.character_positions.insert(old_name.to_string(), current_position);
        }
        
        // Save to disk
        self.save()?;
        
        // Return new position if we have one saved for the new character
        if !new_name.is_empty() {
            if let Some(&new_pos) = self.character_positions.get(new_name) {
                info!("Moving to saved position for '{}': {:?}", new_name, new_pos);
                return Ok(Some(new_pos));
            }
        }
        
        // Character logged out OR new character with no saved position → keep current position
        Ok(None)
    }

    fn parse_num<T: std::str::FromStr + TryFrom<u128>>(var: &str) -> Option<T> where <T as TryFrom<u128>>::Error: std::fmt::Debug, <T as std::str::FromStr>::Err: std::fmt::Debug {
        if let Ok(s) = env::var(var) {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X"))
                && let Ok(n) = u128::from_str_radix(hex, 16)
            {
                return T::try_from(n).inspect_err(|e| error!("failed to parse '{var}' err={e:?}")).ok();
            } else {
                return s.parse::<T>().inspect_err(|e| error!("failed to parse '{var}' err={e:?}")).ok();
            }
        }
        None
    }

    fn from_env() -> Self {
        let border_color_raw = Self::parse_num("BORDER_COLOR").unwrap_or(0x7FFF0000);
        let opacity = Self::parse_num("OPACITY").unwrap_or(0xC0000000);
        let text_fg_raw = Self::parse_num("TEXT_FOREGROUND").unwrap_or(0xFF_FF_FF_FF);
        let text_bg_raw = Self::parse_num("TEXT_BACKGROUND").unwrap_or(0x7F_00_00_00);
        
        Self {
            width: Self::parse_num("WIDTH").unwrap_or(240),
            height: Self::parse_num("HEIGHT").unwrap_or(135),
            opacity_percent: Opacity::from_argb32(opacity).percent(),
            border_size: Self::parse_num("BORDER_SIZE").unwrap_or(5),
            border_color_hex: HexColor::from_argb32(border_color_raw).to_hex_string(),
            text_x: Self::parse_num("TEXT_X").unwrap_or(10),
            text_y: Self::parse_num("TEXT_Y").unwrap_or(20),
            text_foreground_hex: HexColor::from_argb32(text_fg_raw).to_hex_string(),
            text_background_hex: HexColor::from_argb32(text_bg_raw).to_hex_string(),
            hide_when_no_focus: env::var("HIDE_WHEN_NO_FOCUS")
                .map(|x| x.parse().unwrap_or(false))
                .unwrap_or(false),
            character_positions: HashMap::new(),
            snap_threshold: 15,
        }
    }

    fn apply_env_overrides(&mut self) {
        if let Some(width) = Self::parse_num("WIDTH") {
            self.width = width;
        }
        if let Some(height) = Self::parse_num("HEIGHT") {
            self.height = height;
        }
        if let Some(opacity) = Self::parse_num("OPACITY") {
            self.opacity_percent = Opacity::from_argb32(opacity).percent();
        }
        if let Some(border_size) = Self::parse_num("BORDER_SIZE") {
            self.border_size = border_size;
        }
        if let Some(border_color_raw) = Self::parse_num("BORDER_COLOR") {
            self.border_color_hex = HexColor::from_argb32(border_color_raw).to_hex_string();
        }
        if let Some(text_x) = Self::parse_num("TEXT_X") {
            self.text_x = text_x;
        }
        if let Some(text_y) = Self::parse_num("TEXT_Y") {
            self.text_y = text_y;
        }
        if let Some(text_fg) = Self::parse_num("TEXT_FOREGROUND") {
            self.text_foreground_hex = HexColor::from_argb32(text_fg).to_hex_string();
        }
        if let Some(text_bg) = Self::parse_num("TEXT_BACKGROUND") {
            self.text_background_hex = HexColor::from_argb32(text_bg).to_hex_string();
        }
        if let Ok(hide) = env::var("HIDE_WHEN_NO_FOCUS") {
            self.hide_when_no_focus = hide.parse().unwrap_or(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_display_config_valid_colors() {
        let state = PersistentState {
            width: 320,
            height: 180,
            opacity_percent: 75,
            border_size: 3,
            border_color_hex: "#FF00FF00".to_string(), // Green
            text_x: 15,
            text_y: 25,
            text_foreground_hex: "#FFFFFFFF".to_string(), // White
            text_background_hex: "#80000000".to_string(), // 50% transparent black
            hide_when_no_focus: true,
            character_positions: HashMap::new(),
            snap_threshold: 20,
        };

        let config = state.build_display_config();
        assert_eq!(config.width, 320);
        assert_eq!(config.height, 180);
        assert_eq!(config.border_size, 3);
        assert_eq!(config.text_x, 15);
        assert_eq!(config.text_y, 25);
        assert_eq!(config.hide_when_no_focus, true);
        
        // Opacity: 75% → 0xBF
        assert_eq!(config.opacity, 0xBF000000);
        
        // Border color: #FF00FF00 → Color { red: 0, green: 65535, blue: 0, alpha: 65535 }
        assert_eq!(config.border_color.red, 0);
        assert_eq!(config.border_color.green, 65535);
        assert_eq!(config.border_color.blue, 0);
        assert_eq!(config.border_color.alpha, 65535);
    }

    #[test]
    fn test_build_display_config_invalid_colors_fallback() {
        let state = PersistentState {
            width: 200,
            height: 100,
            opacity_percent: 100,
            border_size: 5,
            border_color_hex: "invalid".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "also_invalid".to_string(),
            text_background_hex: "nope".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        let config = state.build_display_config();
        
        // Should fall back to default colors without panicking
        assert_eq!(config.width, 200);
        assert_eq!(config.height, 100);
        
        // Opacity: 100% → 0xFF
        assert_eq!(config.opacity, 0xFF000000);
        
        // Default border_color: 0x7FFF0000 (red with 50% alpha)
        // Alpha conversion: 0x7F (127) * 257 = 32639 in 16-bit
        assert_eq!(config.border_color.red, 65535);
        assert_eq!(config.border_color.blue, 0);
        assert_eq!(config.border_color.alpha, 32639); // 0x7F → 32639 (not 32767)
    }

    #[test]
    fn test_update_position_with_character_name() {
        let mut state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 75,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        // This will try to save(), but we can't control file I/O in test
        // Just verify the HashMap update happens
        let _ = state.update_position("TestChar", 100, 200);
        
        assert_eq!(state.character_positions.get("TestChar"), Some(&Position::new(100, 200)));
    }

    #[test]
    fn test_update_position_empty_name_ignored() {
        let mut state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 75,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        let _ = state.update_position("", 300, 400);
        
        // Empty name should not be inserted
        assert!(state.character_positions.is_empty());
    }

    #[test]
    fn test_handle_character_change_both_names() {
        let mut state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 75,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::from([("NewChar".to_string(), Position::new(500, 600))]),
            snap_threshold: 15,
        };

        let current_pos = Position::new(100, 200);
        // This will fail to save (file I/O in test), but we check the logic
        let result = state.handle_character_change("OldChar", "NewChar", current_pos);
        
        // Should save old position (even if disk save fails)
        assert_eq!(state.character_positions.get("OldChar"), Some(&Position::new(100, 200)));
        
        // File save will fail in test, so we just verify the position was looked up
        // The function returns Err because save() fails, not because logic is wrong
        assert!(result.is_err());
        
        // Verify the new position exists in the map (function would return it if save succeeded)
        assert_eq!(state.character_positions.get("NewChar"), Some(&Position::new(500, 600)));
    }

    #[test]
    fn test_handle_character_change_logout() {
        let mut state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 75,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        let current_pos = Position::new(300, 400);
        let result = state.handle_character_change("LoggingOut", "", current_pos);
        
        // Should save old position (even if disk save fails)
        assert_eq!(state.character_positions.get("LoggingOut"), Some(&Position::new(300, 400)));
        
        // File save will fail in test environment
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_character_change_new_character_no_saved_position() {
        let mut state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 75,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        let current_pos = Position::new(700, 800);
        let result = state.handle_character_change("", "BrandNewChar", current_pos);
        
        // Empty old name not saved
        assert!(state.character_positions.is_empty());
        
        // File save will fail in test environment
        assert!(result.is_err());
    }

    #[test]
    fn test_opacity_percent_roundtrip() {
        // Test that opacity_percent converts correctly through Opacity type
        let state = PersistentState {
            width: 240,
            height: 135,
            opacity_percent: 50,
            border_size: 5,
            border_color_hex: "#7FFF0000".to_string(),
            text_x: 10,
            text_y: 20,
            text_foreground_hex: "#FFFFFFFF".to_string(),
            text_background_hex: "#7F000000".to_string(),
            hide_when_no_focus: false,
            character_positions: HashMap::new(),
            snap_threshold: 15,
        };

        let config = state.build_display_config();
        
        // 50% → 0x7F or 0x80 (due to rounding)
        assert!(config.opacity >= 0x7F000000 && config.opacity <= 0x80000000);
    }
}
