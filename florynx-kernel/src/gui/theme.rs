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
}

pub static DARK: Theme = Theme {
    // PRD: Bioluminescent Glass Desktop — dark luxury futurism
    desktop_top:     Color::rgb(13, 17, 23),   // #0D1117
    desktop_bot:     Color::rgb(22, 29, 41),   // #161D29
    window_bg:       Color::rgb(27, 35, 48),   // #1B2330
    titlebar:        Color::rgb(34, 44, 58),   // #222C3A
    titlebar_active: Color::rgb(43, 54, 70),   // #2B3646
    text:            Color::rgb(243, 247, 250), // #F3F7FA
    text_dim:        Color::rgb(168, 182, 198), // #A8B6C6
    dock_bg:         Color::rgb(17, 23, 35),   // #111723
    dock_icon:       Color::rgb(34, 44, 58),   // #222C3A
    dock_icon_active:Color::rgb(41, 211, 208), // #29D3D0 accent-cyan
    accent:          Color::rgb(41, 211, 208), // #29D3D0 accent-cyan
    border:          Color::rgb(43, 54, 70),   // #2B3646
    shadow:          Color::rgb(5, 7, 11),     // #05070B
    close_btn:       Color::rgb(242, 109, 109), // #F26D6D danger-soft
    minimize_btn:    Color::rgb(217, 255, 114), // #D9FF72 accent-lime
    maximize_btn:    Color::rgb(110, 240, 162), // #6EF0A2 accent-mint
    corner_radius:   10,
    titlebar_h:      34,
    dock_h:          58,
    dock_margin:     14,
    shadow_offset:   4,
    shadow_layers:   3,
    padding:         14,
    menubar_h:       28,
    menubar_bg:      Color::rgba(13, 17, 23, 220),   // semi-transparent dark
    tooltip_bg:      Color::rgb(34, 44, 58),          // #222C3A
    tooltip_text:    Color::rgb(243, 247, 250),       // #F3F7FA
};
