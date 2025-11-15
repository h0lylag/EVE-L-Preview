use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, warn};
use x11rb::protocol::xproto::Window;

/// Maps character names to their window IDs and positions in cycle order
pub struct CycleState {
    /// Configured order from TOML (persistent across sessions)
    config_order: Vec<String>,

    /// Current index in config_order (0-based)
    current_index: usize,

    /// Active windows: character_name → window_id
    /// Only includes characters that currently have windows
    active_windows: HashMap<String, Window>,
}

impl CycleState {
    pub fn new(config_order: Vec<String>) -> Self {
        Self {
            config_order,
            current_index: 0,
            active_windows: HashMap::new(),
        }
    }

    /// Register a new EVE window (called from CreateNotify)
    pub fn add_window(&mut self, character_name: String, window: Window) {
        debug!(
            "Adding window for character '{}': {}",
            character_name, window
        );
        self.active_windows
            .insert(character_name.clone(), window);

        // Add to config order if not present (auto-discovery)
        if !self.config_order.contains(&character_name) {
            self.config_order.push(character_name);
        }
    }

    /// Remove window (called from DestroyNotify)
    pub fn remove_window(&mut self, window: Window) {
        // Find and remove from active_windows
        if let Some((name, _)) = self
            .active_windows
            .iter()
            .find(|&(_, &w)| w == window)
            .map(|(k, v)| (k.clone(), *v))
        {
            debug!("Removing window for character '{}': {}", name, window);
            self.active_windows.remove(&name);

            // If we removed the current character, clamp index
            self.clamp_index();
        }
    }

    /// Update character name (called on login/logout)
    pub fn update_character(&mut self, window: Window, new_name: String) {
        // Remove old entry
        if let Some((old_name, _)) = self
            .active_windows
            .iter()
            .find(|&(_, &w)| w == window)
            .map(|(k, v)| (k.clone(), *v))
        {
            self.active_windows.remove(&old_name);
        }

        // Add new entry
        self.add_window(new_name, window);
    }

    /// Move to next character in config order (Tab)
    /// Returns window to activate, or None if no active characters
    pub fn cycle_forward(&mut self) -> Option<Window> {
        if self.active_windows.is_empty() {
            warn!("No active windows to cycle");
            return None;
        }

        if self.config_order.is_empty() {
            warn!("Config order is empty");
            return None;
        }

        let start_index = self.current_index;
        loop {
            self.current_index = (self.current_index + 1) % self.config_order.len();

            // Found an active character
            if let Some(&window) = self
                .active_windows
                .get(&self.config_order[self.current_index])
            {
                debug!(
                    "Cycling forward to '{}' (index {})",
                    self.config_order[self.current_index], self.current_index
                );
                return Some(window);
            }

            // Wrapped around without finding active character
            if self.current_index == start_index {
                warn!("No active characters found in config order");
                return None;
            }
        }
    }

    /// Move to previous character in config order (Shift+Tab)
    pub fn cycle_backward(&mut self) -> Option<Window> {
        if self.active_windows.is_empty() {
            warn!("No active windows to cycle");
            return None;
        }

        if self.config_order.is_empty() {
            warn!("Config order is empty");
            return None;
        }

        let start_index = self.current_index;
        loop {
            self.current_index = if self.current_index == 0 {
                self.config_order.len() - 1
            } else {
                self.current_index - 1
            };

            // Found an active character
            if let Some(&window) = self
                .active_windows
                .get(&self.config_order[self.current_index])
            {
                debug!(
                    "Cycling backward to '{}' (index {})",
                    self.config_order[self.current_index], self.current_index
                );
                return Some(window);
            }

            // Wrapped around without finding active character
            if self.current_index == start_index {
                warn!("No active characters found in config order");
                return None;
            }
        }
    }

    /// Set current character (called when clicking thumbnail)
    /// Returns true if character exists in config order
    pub fn set_current(&mut self, character_name: &str) -> bool {
        if let Some(index) = self.config_order.iter().position(|c| c == character_name) {
            debug!(
                "Setting current character to '{}' (index {})",
                character_name, index
            );
            self.current_index = index;
            true
        } else {
            warn!("Character '{}' not in config order", character_name);
            false
        }
    }

    /// Clamp index to valid range after removing characters
    fn clamp_index(&mut self) {
        if !self.config_order.is_empty() && self.current_index >= self.config_order.len() {
            self.current_index = 0;
        }
    }

    /// Get current config order for saving
    pub fn config_order(&self) -> &[String] {
        &self.config_order
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cycle_forward_basic() {
        let mut state = CycleState::new(vec![
            "Char1".to_string(),
            "Char2".to_string(),
            "Char3".to_string(),
        ]);

        state.add_window("Char1".to_string(), 100);
        state.add_window("Char2".to_string(), 200);
        state.add_window("Char3".to_string(), 300);

        // Start at index 0 (Char1)
        assert_eq!(state.cycle_forward(), Some(200)); // → Char2
        assert_eq!(state.cycle_forward(), Some(300)); // → Char3
        assert_eq!(state.cycle_forward(), Some(100)); // → Char1 (wrap)
    }

    #[test]
    fn test_cycle_backward_basic() {
        let mut state = CycleState::new(vec![
            "Char1".to_string(),
            "Char2".to_string(),
            "Char3".to_string(),
        ]);

        state.add_window("Char1".to_string(), 100);
        state.add_window("Char2".to_string(), 200);
        state.add_window("Char3".to_string(), 300);

        // Start at index 0 (Char1)
        assert_eq!(state.cycle_backward(), Some(300)); // ← Char3 (wrap)
        assert_eq!(state.cycle_backward(), Some(200)); // ← Char2
        assert_eq!(state.cycle_backward(), Some(100)); // ← Char1
    }

    #[test]
    fn test_set_current() {
        let mut state = CycleState::new(vec!["Char1".to_string(), "Char2".to_string()]);

        state.add_window("Char1".to_string(), 100);
        state.add_window("Char2".to_string(), 200);

        assert!(state.set_current("Char2"));
        assert_eq!(state.cycle_forward(), Some(100)); // Next after Char2 is Char1
    }

    #[test]
    fn test_skip_inactive_characters() {
        let mut state = CycleState::new(vec![
            "Active1".to_string(),
            "Inactive".to_string(),
            "Active2".to_string(),
        ]);

        state.add_window("Active1".to_string(), 100);
        state.add_window("Active2".to_string(), 300);
        // "Inactive" not added

        // Should skip "Inactive" in cycle
        assert_eq!(state.cycle_forward(), Some(300)); // Active1 → Active2
        assert_eq!(state.cycle_forward(), Some(100)); // Active2 → Active1 (wrap, skip Inactive)
    }

    #[test]
    fn test_remove_current_character() {
        let mut state = CycleState::new(vec!["Char1".to_string(), "Char2".to_string()]);

        state.add_window("Char1".to_string(), 100);
        state.add_window("Char2".to_string(), 200);

        state.set_current("Char2");
        state.remove_window(200); // Remove current character

        // Index should be clamped and cycle should still work
        assert_eq!(state.cycle_forward(), Some(100));
    }

    #[test]
    fn test_empty_order() {
        let mut state = CycleState::new(vec![]);
        assert_eq!(state.cycle_forward(), None);
        assert_eq!(state.cycle_backward(), None);
    }

    #[test]
    fn test_auto_add_new_character() {
        let mut state = CycleState::new(vec!["Char1".to_string()]);

        state.add_window("Char1".to_string(), 100);
        state.add_window("NewChar".to_string(), 200);

        // NewChar should be auto-added to config order
        assert_eq!(state.config_order.len(), 2);
        assert!(state.config_order.contains(&"NewChar".to_string()));
    }

    #[test]
    fn test_update_character_name() {
        let mut state = CycleState::new(vec!["OldName".to_string()]);

        state.add_window("OldName".to_string(), 100);
        state.update_character(100, "NewName".to_string());

        // Old name should be removed, new name added
        assert!(!state.active_windows.contains_key("OldName"));
        assert_eq!(state.active_windows.get("NewName"), Some(&100));
    }
}
