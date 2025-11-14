//! Domain types for type safety and clarity

use serde::{Deserialize, Serialize};

/// A position in 2D space (X11 coordinates)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct Position {
    pub x: i16,
    pub y: i16,
}

impl Position {
    /// Create a new position
    pub fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }

    /// Convert to tuple for compatibility
    pub fn as_tuple(self) -> (i16, i16) {
        (self.x, self.y)
    }

    /// Create from tuple
    pub fn from_tuple(tuple: (i16, i16)) -> Self {
        Self { x: tuple.0, y: tuple.1 }
    }
}

impl From<(i16, i16)> for Position {
    fn from(tuple: (i16, i16)) -> Self {
        Self::from_tuple(tuple)
    }
}

impl From<Position> for (i16, i16) {
    fn from(pos: Position) -> Self {
        pos.as_tuple()
    }
}

/// Per-character settings: position and thumbnail dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterSettings {
    pub x: i16,
    pub y: i16,
    /// Thumbnail width (0 = use auto-detect)
    #[serde(default)]
    pub width: u16,
    /// Thumbnail height (0 = use auto-detect)
    #[serde(default)]
    pub height: u16,
}

impl CharacterSettings {
    pub fn new(x: i16, y: i16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }
    
    pub fn position(&self) -> Position {
        Position::new(self.x, self.y)
    }
    
    pub fn dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_creation() {
        let pos = Position::new(100, 200);
        assert_eq!(pos.x, 100);
        assert_eq!(pos.y, 200);
    }

    #[test]
    fn test_position_tuple_conversion() {
        let pos = Position::new(150, 250);
        let tuple = pos.as_tuple();
        assert_eq!(tuple, (150, 250));
        
        let pos2 = Position::from_tuple(tuple);
        assert_eq!(pos, pos2);
    }

    #[test]
    fn test_position_from_trait() {
        let pos: Position = (100, 200).into();
        assert_eq!(pos.x, 100);
        assert_eq!(pos.y, 200);
        
        let tuple: (i16, i16) = pos.into();
        assert_eq!(tuple, (100, 200));
    }
}
