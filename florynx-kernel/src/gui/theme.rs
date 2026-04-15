// =============================================================================
// Florynx Kernel — GUI Theme
// =============================================================================
// Dark-mode color palette and spacing constants inspired by macOS/Windows 11.
// =============================================================================

use crate::gui::renderer::Color;

pub struct Theme {
    pub desktop_top: Color,
    pub desktop_bot: Color,
    pub window_bg: Color,
    pub titlebar: Color,
    pub titlebar_active: Color,
    pub text: Color,
    pub text_dim: Color,
    pub dock_bg: Color,
    pub dock_icon: Color,
    pub dock_icon_active: Color,
    pub accent: Color,
    pub border: Color,
    pub shadow: Color,
    pub close_btn: Color,
    pub minimize_btn: Color,
    pub maximize_btn: Color,
    pub corner_radius: usize,
    pub titlebar_h: usize,
    pub dock_h: usize,
    pub dock_margin: usize,
    pub shadow_offset: usize,
    pub shadow_layers: usize,
    pub padding: usize,
    pub menubar_h: usize,
    pub menubar_bg: Color,
    pub tooltip_bg: Color,
    pub tooltip_text: Color,
    // Search bar (menubar)
    pub search_bar_bg: Color,
    pub search_bar_text: Color,
    // Snap preview
    pub snap_preview: Color,
    // Resize constraints
    pub resize_grab: usize,
    pub min_window_w: usize,
    pub min_window_h: usize,
    // Selection highlight
    pub selection_bg: Color,
}

pub static DARK: Theme = Theme {
    // PRD: Bioluminescent Glass Desktop — dark luxury futurism
    desktop_top:     Color::rgb(13, 17, 23),   // #0D1117
    desktop_bot:     Color::rgb(22, 29, 41),   // #161D29
    window_bg:       Color::rgb(27, 35, 48),   // #1B2330
    titlebar:        Color::rgb(34, 44, 58),   // #222C3A
    titlebar_active: Color::rgb(43, 54, 70),   // #2B3646
    text:            Color::rgb(255, 255, 255), // Pure White
    text_dim:        Color::rgb(210, 220, 230), // Lighter dim for contrast
    dock_bg:         Color::rgba(255, 255, 255, 45), // Translucent Bright Glass
    dock_icon:       Color::rgb(240, 248, 255),  // Bright Alice Blue
    dock_icon_active:Color::rgb(41, 211, 208), // #29D3D0 accent-cyan
    accent:          Color::rgb(41, 211, 208), // #29D3D0 accent-cyan
    border:          Color::rgba(255, 255, 255, 80), // Frosty glass border
    shadow:          Color::rgba(0, 0, 0, 100),// Dark glass shadow
    close_btn:       Color::rgb(242, 109, 109), // #F26D6D danger-soft
    minimize_btn:    Color::rgb(217, 255, 114), // #D9FF72 accent-lime
    maximize_btn:    Color::rgb(110, 240, 162), // #6EF0A2 accent-mint
    corner_radius:   14,                       // More rounded
    titlebar_h:      34,
    dock_h:          76,                       // Taller dock for larger icons
    dock_margin:     16,
    shadow_offset:   4,
    shadow_layers:   3,
    padding:         18,
    menubar_h:       32,                       // slightly taller menubar
    menubar_bg:      Color::TRANSPARENT,       // Fully transparent floating menubar
    tooltip_bg:      Color::rgba(20, 25, 35, 200), // Glassy dark tooltip
    tooltip_text:    Color::rgb(243, 247, 250),
    // Search bar
    search_bar_bg:   Color::rgba(80, 85, 95, 160),
    search_bar_text: Color::rgb(130, 138, 150),
    // Snap preview
    snap_preview:    Color::rgba(41, 211, 208, 40),
    // Resize constraints
    resize_grab:     6,
    min_window_w:    160,
    min_window_h:    100,
    // Selection highlight
    selection_bg:    Color::rgba(41, 211, 208, 60),
};
