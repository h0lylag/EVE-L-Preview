use anyhow::Result;
use std::collections::HashMap;
use tracing::info;
use x11rb::protocol::xproto::Window;

/// Runtime state for position tracking
/// Window positions are session-only (not persisted to disk)
pub struct SavedState {
    /// Window ID → (x, y) position (session-only, not persisted)
    /// Used for logged-out windows that show "EVE" without character name
    /// Window IDs are ephemeral and don't survive X11 server restarts
    pub window_positions: HashMap<Window, (i16, i16)>,
    
    /// TODO: Move to PersistentState - behavior for new characters on existing windows
    /// - false: New character spawns centered (current behavior)
    /// - true: New character inherits window's last position
    pub inherit_window_position: bool,
}

impl Default for SavedState {
    fn default() -> Self {
        Self {
            window_positions: HashMap::new(),
            inherit_window_position: false,
        }
    }
}

impl SavedState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get initial position for a thumbnail
    /// Priority: character position (from persistent state) > window position (if enabled) > None (use center)
    /// Window position only used for logged-out windows or if inherit_window_position is enabled
    pub fn get_position(
        &self,
        character_name: &str,
        window: Window,
        character_positions: &HashMap<String, (i16, i16)>,
    ) -> Option<(i16, i16)> {
        // If character has a name (not just "EVE"), check character position from config
        if !character_name.is_empty() {
            if let Some(&pos) = character_positions.get(character_name) {
                info!("Using saved position for character '{}': {:?}", character_name, pos);
                return Some(pos);
            }
            
            // TODO: When config option is added, check inherit_window_position here
            // For now, new character always spawns centered
            if self.inherit_window_position {
                if let Some(&pos) = self.window_positions.get(&window) {
                    info!("Inheriting window position for new character '{}': {:?}", character_name, pos);
                    return Some(pos);
                }
            }
            
            // New character with no saved position → return None (will center)
            return None;
        }
        
        // Logged-out window ("EVE" title) → use window position from this session
        if let Some(&pos) = self.window_positions.get(&window) {
            info!("Using session position for logged-out window {}: {:?}", window, pos);
            Some(pos)
        } else {
            None
        }
    }

    /// Update session position (window tracking)
    pub fn update_window_position(&mut self, window: Window, x: i16, y: i16) {
        self.window_positions.insert(window, (x, y));
        info!("Saved session position for window {}: ({}, {})", window, x, y);
    }
}
