// =============================================================================
// Florynx Userland — Theme: Breeze Bioluminescent
// =============================================================================
// KDE Plasma Breeze Dark-inspired theme adapted with Florynx's bioluminescent
// green/cyan accents. Clean, modern, professional.
// =============================================================================

/// RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    pub const BLACK: Color = Color::rgb(0, 0, 0);
}

// ---------------------------------------------------------------------------
// Breeze Bioluminescent color palette
// ---------------------------------------------------------------------------

/// Panel background (KDE Plasma-style frosted dark)
pub const PANEL_BG: Color = Color::rgba(24, 28, 36, 230);
/// Panel border (subtle separator)
pub const PANEL_BORDER: Color = Color::rgba(48, 52, 60, 200);
/// Panel height in pixels
pub const PANEL_HEIGHT: usize = 40;

/// Desktop background fallback (used if no wallpaper loaded)
pub const DESKTOP_BG_TOP: Color = Color::rgb(12, 16, 24);
pub const DESKTOP_BG_BOT: Color = Color::rgb(8, 24, 20);

/// Window colors (Breeze Dark)
pub const WINDOW_BG: Color = Color::rgb(35, 38, 46);
pub const WINDOW_HEADER: Color = Color::rgb(42, 46, 54);
pub const WINDOW_HEADER_ACTIVE: Color = Color::rgb(48, 52, 62);
pub const WINDOW_BORDER: Color = Color::rgba(60, 64, 72, 180);
pub const WINDOW_SHADOW: Color = Color::rgba(0, 0, 0, 60);
pub const WINDOW_CORNER_RADIUS: usize = 10;

/// Titlebar buttons (KDE-style, subtle)
pub const BTN_CLOSE: Color = Color::rgb(200, 55, 55);
pub const BTN_MAXIMIZE: Color = Color::rgb(50, 160, 80);
pub const BTN_MINIMIZE: Color = Color::rgb(200, 170, 50);

/// Text colors
pub const TEXT_PRIMARY: Color = Color::rgb(230, 235, 240);
pub const TEXT_SECONDARY: Color = Color::rgb(160, 168, 178);
pub const TEXT_DISABLED: Color = Color::rgb(100, 106, 115);

/// Accent color (Florynx bioluminescent cyan-green)
pub const ACCENT: Color = Color::rgb(41, 211, 208);
pub const ACCENT_HOVER: Color = Color::rgb(60, 230, 225);
pub const ACCENT_GREEN: Color = Color::rgb(110, 240, 162);

/// App menu / Kickoff-style launcher
pub const MENU_BG: Color = Color::rgba(28, 32, 40, 240);
pub const MENU_ITEM_HOVER: Color = Color::rgba(41, 211, 208, 40);
pub const MENU_SEPARATOR: Color = Color::rgba(60, 64, 72, 100);

/// System tray
pub const SYSTRAY_TEXT: Color = Color::rgb(190, 198, 208);
pub const SYSTRAY_ICON: Color = Color::rgb(180, 188, 198);

/// Taskbar
pub const TASKBAR_ACTIVE: Color = Color::rgba(41, 211, 208, 50);
pub const TASKBAR_HOVER: Color = Color::rgba(255, 255, 255, 20);
pub const TASKBAR_INDICATOR: Color = ACCENT;

/// Selection / highlight
pub const SELECTION_BG: Color = Color::rgba(41, 211, 208, 80);
pub const HOVER_BG: Color = Color::rgba(255, 255, 255, 15);

/// Tooltip
pub const TOOLTIP_BG: Color = Color::rgba(48, 52, 60, 240);
pub const TOOLTIP_TEXT: Color = TEXT_PRIMARY;

/// UI metrics
pub const TITLEBAR_HEIGHT: usize = 32;
pub const CORNER_RADIUS: usize = 10;
pub const SHADOW_LAYERS: usize = 3;
pub const PADDING: usize = 10;
pub const ICON_SIZE_SMALL: usize = 16;
pub const ICON_SIZE_MEDIUM: usize = 24;
pub const ICON_SIZE_LARGE: usize = 48;

/// Font metrics (8x8 bitmap font, scale 1)
pub const FONT_WIDTH: usize = 8;
pub const FONT_HEIGHT: usize = 8;
pub const LINE_HEIGHT: usize = 12;

// ---------------------------------------------------------------------------
// Wallpaper list (built-in default wallpapers)
// ---------------------------------------------------------------------------

/// Default wallpaper index.
pub const DEFAULT_WALLPAPER: usize = 1;

/// Wallpaper filenames (stored in assets/wallpapers/).
pub const WALLPAPER_NAMES: &[&str] = &[
    "background 1.webp",  // Bioluminescent crystals
    "background 2.webp",  // Flowing waves (default)
    "background 3.webp",  // Nebula
];
