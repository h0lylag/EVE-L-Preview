# eve-l-preview

An X11 EVE-O Preview lookalike. Works flawlessly on Wayland as long as you run Wine/Proton through XWayland (default behaviour).

## Features

- Highlight border for the active EVE client
- Left-click to focus a client
- Right-click and drag to reposition thumbnails
- **Tab/Shift+Tab hotkeys to cycle through clients**
- Character name overlay
- Per-character thumbnail positions and dimensions
- Optional hide-when-unfocused mode
- Edge and corner snapping when dragging
- Extremely lightweight (<1 MiB RAM)
- Fully configurable via TOML config file + environment variable overrides

## Configuration

Configuration is loaded from `~/.config/eve-l-preview/eve-l-preview.toml` (auto-generated on first run).
Environment variables override TOML values for quick testing.

### TOML Configuration

```toml
# Global display settings
opacity_percent = 75
border_size = 3
border_color = "#7FFF0000"
text_x = 10
text_y = 20
text_foreground = "#FFFFFFFF"
text_background = "#7F000000"
hide_when_no_focus = false
snap_threshold = 15

# Hotkey cycling order (Tab/Shift+Tab)
# Edit this list with your actual character names!
# The order here determines Tab/Shift+Tab cycling order
hotkey_order = ["Main Character", "Alt 1", "Alt 2"]

# Per-character settings
[characters."Main"]
x = 100
y = 200
width = 480
height = 270

[characters."Alt1"]
x = 600
y = 200
width = 480
height = 270
```

### Configuration Options

| Setting | Type | Default | Description |
|-----------|------|----------|-------------|
| `opacity_percent` | u8 | 75 | Thumbnail opacity (0-100%) |
| `border_size` | u16 | 5 | Border width in pixels |
| `border_color` | Hex | `#7FFF0000` | Border color (ARGB format) |
| `text_x` | i16 | 10 | Character name X offset  |
| `text_y` | i16 | 20 | Character name Y offset |
| `text_foreground` | Hex | `#FFFFFFFF` | Text color (ARGB format) |
| `text_background` | Hex | `#7F000000` | Text background (ARGB format) |
| `hide_when_no_focus` | bool | false | Hide thumbnails when all clients unfocused |
| `snap_threshold` | u16 | 15 | Snap distance in pixels (0 = disabled) |
| `hotkey_order` | Array | `[]` | Character order for Tab cycling |

**Per-character settings** (saved automatically on drag):
- `x`, `y` - Window position
- `width`, `height` - Thumbnail dimensions

### Environment Variable Overrides

All global settings can be overridden via environment variables:

```bash
OPACITY=0xC0000000 BORDER_COLOR=0xFF00FF00 eve-l-preview
```

| Variable | Type | Description |
|-----------|------|-------------|
| `OPACITY` | u32 | Thumbnail opacity (ARGB format) |
| `BORDER_SIZE` | u16 | Border width |
| `BORDER_COLOR` | u32 | Border color (ARGB hex) |
| `TEXT_X` | i16 | Text X position  |
| `TEXT_Y` | i16 | Text Y position |
| `TEXT_FOREGROUND` | u32 | Text color (ARGB hex) |
| `TEXT_BACKGROUND` | u32 | Text background (ARGB hex) |
| `HIDE_WHEN_NO_FOCUS` | bool | Hide when unfocused |

> Colors support both hex (`0xAARRGGBB` or `#AARRGGBB`) and decimal formats.

## Usage

Run before or after launching EVE clients. The application uses minimal resources, especially under XWayland.

### Hotkey Controls

**Tab/Shift+Tab cycling requires `input` group membership:**

```bash
sudo usermod -a -G input $USER
# Log out and back in for changes to take effect
```

Once configured:
- **Tab** - Cycle forward through clients
- **Shift+Tab** - Cycle backward through clients
- **Left-click thumbnail** - Focus that client and set as current for next Tab press
- **Right-click and drag** - Reposition thumbnail

The cycle order follows the `hotkey_order` array in your config. Characters are auto-added when detected, but you can manually reorder them. Clicking a thumbnail makes it the current position for the next Tab press.

**If hotkey permissions are not set up**, the application continues without hotkey support (click-to-focus still works).

### Logging

Set log level for debugging:

```bash
LOG_LEVEL=debug eve-l-preview
```

Valid levels: `trace`, `debug`, `info` (default), `warn`, `error`
