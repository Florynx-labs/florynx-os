// =============================================================================
// Florynx Userland — App Menu (Kickoff-Style Launcher)
// =============================================================================
// KDE Kickoff-inspired application launcher.
// Appears when clicking the app menu button on the panel.
// =============================================================================

/// Application entry in the launcher.
#[derive(Debug, Clone, Copy)]
pub struct AppEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub icon_idx: usize,
    pub category: AppCategory,
}

/// Application categories (like KDE Kickoff tabs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCategory {
    Favorites,
    System,
    Utilities,
    Development,
}

/// Default application entries.
pub const DEFAULT_APPS: &[AppEntry] = &[
    AppEntry { name: "Files",          description: "File Manager",     icon_idx: 0, category: AppCategory::Favorites },
    AppEntry { name: "Terminal",       description: "Terminal Emulator", icon_idx: 1, category: AppCategory::System },
    AppEntry { name: "Settings",       description: "System Settings",  icon_idx: 2, category: AppCategory::System },
    AppEntry { name: "System Monitor", description: "Resource Monitor",  icon_idx: 3, category: AppCategory::System },
    AppEntry { name: "Notes",          description: "Text Editor",      icon_idx: 4, category: AppCategory::Utilities },
];

/// App menu state.
pub struct AppMenu {
    pub visible: bool,
    pub selected: Option<usize>,
    pub active_category: AppCategory,
    pub menu_x: usize,
    pub menu_y: usize,
    pub menu_w: usize,
    pub menu_h: usize,
}

impl AppMenu {
    pub fn new(screen_w: usize, screen_h: usize, panel_height: usize) -> Self {
        let w = 340;
        let h = 420;
        AppMenu {
            visible: false,
            selected: None,
            active_category: AppCategory::Favorites,
            menu_x: 0,
            menu_y: screen_h.saturating_sub(panel_height + h),
            menu_w: w,
            menu_h: h,
        }
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
        if !self.visible {
            self.selected = None;
        }
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.selected = None;
    }

    /// Get apps for the active category.
    pub fn filtered_apps(&self) -> alloc::vec::Vec<&'static AppEntry> {
        DEFAULT_APPS.iter()
            .filter(|a| a.category == self.active_category || self.active_category == AppCategory::Favorites)
            .collect()
    }

    /// Check if a point is inside the menu.
    pub fn contains(&self, x: usize, y: usize) -> bool {
        self.visible
            && x >= self.menu_x && x < self.menu_x + self.menu_w
            && y >= self.menu_y && y < self.menu_y + self.menu_h
    }
}
