// =============================================================================
// Florynx Userland — System Settings (KDE System Settings-Style)
// =============================================================================

/// Settings categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsPage {
    Appearance,
    Wallpaper,
    Display,
    Keyboard,
    Mouse,
    About,
}

/// Settings application state.
pub struct Settings {
    pub active_page: SettingsPage,
    pub sidebar_width: usize,
}

impl Settings {
    pub fn new() -> Self {
        Settings {
            active_page: SettingsPage::Appearance,
            sidebar_width: 200,
        }
    }
}
