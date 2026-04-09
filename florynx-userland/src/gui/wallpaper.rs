// =============================================================================
// Florynx Userland — Wallpaper Manager
// =============================================================================
// Manages desktop wallpaper selection and rendering.
// Default wallpapers are Florynx bioluminescent themes.
// =============================================================================

use super::theme;

/// Wallpaper rendering mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WallpaperMode {
    /// Stretch to fill screen.
    Stretch,
    /// Center, letterbox with desktop_bg.
    Center,
    /// Tile the image.
    Tile,
    /// Use procedural gradient (fallback).
    Gradient,
}

/// Wallpaper state.
pub struct WallpaperManager {
    /// Index into theme::WALLPAPER_NAMES.
    pub current: usize,
    /// Rendering mode.
    pub mode: WallpaperMode,
    /// Screen dimensions.
    pub screen_w: usize,
    pub screen_h: usize,
}

impl WallpaperManager {
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        WallpaperManager {
            current: theme::DEFAULT_WALLPAPER,
            mode: WallpaperMode::Stretch,
            screen_w,
            screen_h,
        }
    }

    /// Get the current wallpaper filename.
    pub fn current_name(&self) -> &'static str {
        if self.current < theme::WALLPAPER_NAMES.len() {
            theme::WALLPAPER_NAMES[self.current]
        } else {
            theme::WALLPAPER_NAMES[0]
        }
    }

    /// Cycle to the next wallpaper.
    pub fn next(&mut self) {
        self.current = (self.current + 1) % theme::WALLPAPER_NAMES.len();
    }

    /// Cycle to the previous wallpaper.
    pub fn prev(&mut self) {
        if self.current == 0 {
            self.current = theme::WALLPAPER_NAMES.len() - 1;
        } else {
            self.current -= 1;
        }
    }

    /// Set wallpaper by index.
    pub fn set(&mut self, index: usize) {
        if index < theme::WALLPAPER_NAMES.len() {
            self.current = index;
        }
    }
}
