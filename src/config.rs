use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::{error, info};
use x11rb::protocol::render::Color;

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
    /// Character name → (x, y) position
    /// Persisted positions for each character's thumbnail
    #[serde(default)]
    pub character_positions: HashMap<String, (i16, i16)>,
    
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
        // Parse colors from hex strings
        let border_color = Self::parse_hex_color(&self.border_color_hex)
            .map(Self::u32_to_color)
            .unwrap_or_else(|| {
                error!("Invalid border_color hex, using default");
                Self::u32_to_color(0x7FFF0000)
            });
        
        let text_foreground = Self::parse_hex_color(&self.text_foreground_hex)
            .map(Self::premultiply_argb32)
            .unwrap_or_else(|| {
                error!("Invalid text_foreground hex, using default");
                Self::premultiply_argb32(0xFF_FF_FF_FF)
            });
        
        let text_background = Self::parse_hex_color(&self.text_background_hex)
            .map(Self::premultiply_argb32)
            .unwrap_or_else(|| {
                error!("Invalid text_background hex, using default");
                Self::premultiply_argb32(0x7F_00_00_00)
            });
        
        let opacity = Self::percent_to_opacity(self.opacity_percent);
        
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
            self.character_positions.insert(character_name.to_string(), (x, y));
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
        current_position: (i16, i16),
    ) -> Result<Option<(i16, i16)>> {
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

    fn u32_to_color(raw: u32) -> Color {
        let a = ((raw >> 24) & 0xFF) as u16;
        let r = ((raw >> 16) & 0xFF) as u16;
        let g = ((raw >> 8) & 0xFF) as u16;
        let b = (raw & 0xFF) as u16;

        let scale = |v: u16| (v as f32 / u8::MAX as f32 * u16::MAX as f32) as u16;

        Color {
            red: scale(r),
            green: scale(g),
            blue: scale(b),
            alpha: scale(a),
        }
    }

    fn parse_hex_color(hex: &str) -> Option<u32> {
        let hex = hex.strip_prefix('#')?;
        u32::from_str_radix(hex, 16).ok()
    }

    fn u32_to_hex_color(color: u32) -> String {
        format!("#{:08X}", color)
    }

    fn percent_to_opacity(percent: u8) -> u32 {
        let percent = percent.min(100);
        let alpha = (percent as f32 / 100.0 * 255.0) as u32;
        alpha << 24
    }

    fn opacity_to_percent(opacity: u32) -> u8 {
        let alpha = (opacity >> 24) & 0xFF;
        ((alpha as f32 / 255.0) * 100.0) as u8
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

    fn premultiply_argb32(argb: u32) -> u32 {
        let a = (argb >> 24) & 0xFF;
        let r = (argb >> 16) & 0xFF;
        let g = (argb >> 8) & 0xFF;
        let b = argb & 0xFF;

        let r_p = r * a / 255;
        let g_p = g * a / 255;
        let b_p = b * a / 255;

        (a << 24) | (r_p << 16) | (g_p << 8) | b_p
    }

    fn from_env() -> Self {
        let border_color_raw = Self::parse_num("BORDER_COLOR").unwrap_or(0x7FFF0000);
        let opacity = Self::parse_num("OPACITY").unwrap_or(0xC0000000);
        let text_fg_raw = Self::parse_num("TEXT_FOREGROUND").unwrap_or(0xFF_FF_FF_FF);
        let text_bg_raw = Self::parse_num("TEXT_BACKGROUND").unwrap_or(0x7F_00_00_00);
        
        Self {
            width: Self::parse_num("WIDTH").unwrap_or(240),
            height: Self::parse_num("HEIGHT").unwrap_or(135),
            opacity_percent: Self::opacity_to_percent(opacity),
            border_size: Self::parse_num("BORDER_SIZE").unwrap_or(5),
            border_color_hex: Self::u32_to_hex_color(border_color_raw),
            text_x: Self::parse_num("TEXT_X").unwrap_or(10),
            text_y: Self::parse_num("TEXT_Y").unwrap_or(20),
            text_foreground_hex: Self::u32_to_hex_color(text_fg_raw),
            text_background_hex: Self::u32_to_hex_color(text_bg_raw),
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
            self.opacity_percent = Self::opacity_to_percent(opacity);
        }
        if let Some(border_size) = Self::parse_num("BORDER_SIZE") {
            self.border_size = border_size;
        }
        if let Some(border_color_raw) = Self::parse_num("BORDER_COLOR") {
            self.border_color_hex = Self::u32_to_hex_color(border_color_raw);
        }
        if let Some(text_x) = Self::parse_num("TEXT_X") {
            self.text_x = text_x;
        }
        if let Some(text_y) = Self::parse_num("TEXT_Y") {
            self.text_y = text_y;
        }
        if let Some(text_fg) = Self::parse_num("TEXT_FOREGROUND") {
            self.text_foreground_hex = Self::u32_to_hex_color(text_fg);
        }
        if let Some(text_bg) = Self::parse_num("TEXT_BACKGROUND") {
            self.text_background_hex = Self::u32_to_hex_color(text_bg);
        }
        if let Ok(hide) = env::var("HIDE_WHEN_NO_FOCUS") {
            self.hide_when_no_focus = hide.parse().unwrap_or(false);
        }
    }
}
