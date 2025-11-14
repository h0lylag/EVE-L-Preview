use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;
use tracing::error;
use x11rb::protocol::render::Color;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub width: u16,
    pub height: u16,
    pub opacity: u32,
    pub border_size: u16,
    #[serde(skip)]
    pub border_color: Color,
    #[serde(rename = "border_color")]
    border_color_raw: u32,
    pub text_x: i16,
    pub text_y: i16,
    pub text_foreground: u32,
    pub text_background: u32,
    pub hide_when_no_focus: bool,
}

impl Config {
    fn config_path() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("eve-l-preview");
        path.push("eve-l-preview.toml");
        path
    }

    pub fn load() -> Self {
        // Try to load existing config file
        let config_path = Self::config_path();
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(mut config) = toml::from_str::<Config>(&contents) {
                // Convert raw color to Color struct
                config.border_color = Self::u32_to_color(config.border_color_raw);
                
                // Apply env var overrides
                config.apply_env_overrides();
                return config;
            }
        }

        // Generate new config from env vars
        let config = Self::from_env();
        
        // Save for next time
        if let Err(e) = config.save() {
            error!("Failed to save config: {e:?}");
        } else {
            println!("Generated config file: {}", config_path.display());
            println!("Edit it to customize settings (env vars still override)");
        }
        
        config
    }

    fn save(&self) -> Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = toml::to_string_pretty(self)?;
        fs::write(&path, contents)?;
        Ok(())
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
        
        Self {
            width: Self::parse_num("WIDTH").unwrap_or(240),
            height: Self::parse_num("HEIGHT").unwrap_or(135),
            opacity: Self::parse_num("OPACITY").unwrap_or(0xC0000000),
            border_size: Self::parse_num("BORDER_SIZE").unwrap_or(5),
            border_color: Self::u32_to_color(border_color_raw),
            border_color_raw,
            text_x: Self::parse_num("TEXT_X").unwrap_or(10),
            text_y: Self::parse_num("TEXT_Y").unwrap_or(125),
            text_foreground: Self::premultiply_argb32(
                Self::parse_num("TEXT_FOREGROUND").unwrap_or(0xFF_FF_FF_FF),
            ),
            text_background: Self::premultiply_argb32(
                Self::parse_num("TEXT_BACKGROUND").unwrap_or(0x7F_00_00_00),
            ),
            hide_when_no_focus: env::var("HIDE_WHEN_NO_FOCUS")
                .map(|x| x.parse().unwrap_or(false))
                .unwrap_or(false),
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
            self.opacity = opacity;
        }
        if let Some(border_size) = Self::parse_num("BORDER_SIZE") {
            self.border_size = border_size;
        }
        if let Some(border_color_raw) = Self::parse_num("BORDER_COLOR") {
            self.border_color_raw = border_color_raw;
            self.border_color = Self::u32_to_color(border_color_raw);
        }
        if let Some(text_x) = Self::parse_num("TEXT_X") {
            self.text_x = text_x;
        }
        if let Some(text_y) = Self::parse_num("TEXT_Y") {
            self.text_y = text_y;
        }
        if let Some(text_fg) = Self::parse_num("TEXT_FOREGROUND") {
            self.text_foreground = Self::premultiply_argb32(text_fg);
        }
        if let Some(text_bg) = Self::parse_num("TEXT_BACKGROUND") {
            self.text_background = Self::premultiply_argb32(text_bg);
        }
        if let Ok(hide) = env::var("HIDE_WHEN_NO_FOCUS") {
            self.hide_when_no_focus = hide.parse().unwrap_or(false);
        }
    }
}
