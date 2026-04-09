// =============================================================================
// Florynx Userland — Panel (KDE Plasma-Style)
// =============================================================================
// Bottom panel with: [App Menu] [Taskbar] [System Tray + Clock]
// Inspired by KDE Plasma 6 default layout.
// =============================================================================

use super::theme;

/// Panel position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPosition {
    Bottom,
    Top,
}

/// Panel layout zones.
#[derive(Debug, Clone, Copy)]
pub struct PanelLayout {
    /// App menu button area.
    pub menu_x: usize,
    pub menu_w: usize,
    /// Taskbar area (between menu and systray).
    pub taskbar_x: usize,
    pub taskbar_w: usize,
    /// System tray area (right side).
    pub systray_x: usize,
    pub systray_w: usize,
}

/// The desktop panel.
pub struct Panel {
    pub position: PanelPosition,
    pub height: usize,
    pub screen_w: usize,
    pub screen_h: usize,
    pub layout: PanelLayout,
    pub visible: bool,
}

impl Panel {
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        let h = theme::PANEL_HEIGHT;
        // Layout: [40px menu] [flex taskbar] [200px systray]
        let menu_w = 44;
        let systray_w = 200;
        let taskbar_x = menu_w;
        let taskbar_w = screen_w.saturating_sub(menu_w + systray_w);
        let systray_x = screen_w.saturating_sub(systray_w);

        Panel {
            position: PanelPosition::Bottom,
            height: h,
            screen_w,
            screen_h,
            layout: PanelLayout {
                menu_x: 0,
                menu_w,
                taskbar_x,
                taskbar_w,
                systray_x,
                systray_w,
            },
            visible: true,
        }
    }

    /// Panel y position on screen.
    pub fn y(&self) -> usize {
        match self.position {
            PanelPosition::Bottom => self.screen_h.saturating_sub(self.height),
            PanelPosition::Top => 0,
        }
    }

    /// Full panel rectangle.
    pub fn bounds(&self) -> (usize, usize, usize, usize) {
        (0, self.y(), self.screen_w, self.height)
    }

    /// Check if a point is inside the panel.
    pub fn contains(&self, x: usize, y: usize) -> bool {
        let py = self.y();
        x < self.screen_w && y >= py && y < py + self.height
    }

    /// Which zone was clicked: 0=menu, 1=taskbar, 2=systray.
    pub fn hit_zone(&self, x: usize) -> u8 {
        if x < self.layout.menu_w { 0 }
        else if x < self.layout.systray_x { 1 }
        else { 2 }
    }
}
