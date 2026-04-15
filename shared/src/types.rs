// =============================================================================
// Florynx Shared — Common Types
// =============================================================================

/// Rectangle used for dirty regions, window bounds, etc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
}

/// Color (RGBA).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// GUI event passed from kernel to userland.
#[derive(Debug, Clone, Copy)]
pub enum GuiEvent {
    MouseMove { x: u32, y: u32 },
    MouseDown { x: u32, y: u32, button: u8 },
    MouseUp { x: u32, y: u32, button: u8 },
    KeyPress { scancode: u8, ascii: u8 },
    KeyRelease { scancode: u8 },
    WindowFocus { win_id: u32 },
    WindowClose { win_id: u32 },
    Resize { w: u32, h: u32 },
}

/// Window creation parameters.
#[derive(Debug, Clone, Copy)]
pub struct WindowParams {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub flags: u32,
}

/// Window flags.
pub const WIN_FLAG_DECORATED: u32 = 1 << 0;
pub const WIN_FLAG_RESIZABLE: u32 = 1 << 1;
pub const WIN_FLAG_PANEL: u32 = 1 << 2;      // KDE-style panel (no decoration)
pub const WIN_FLAG_DOCK: u32 = 1 << 3;       // Dock/taskbar
pub const WIN_FLAG_POPUP: u32 = 1 << 4;      // Popup menu
pub const WIN_FLAG_DESKTOP: u32 = 1 << 5;    // Desktop background layer

/// Process ID type.
pub type Pid = u32;

/// Window ID type.
pub type WinId = u32;

// =============================================================================
// HGUI Kernel/Userland Link Constants
// =============================================================================

pub const HGUI_SCREEN_W: u32 = 1024;
pub const HGUI_SCREEN_H: u32 = 768;
pub const HGUI_PANEL_HEIGHT: u32 = 40;

pub const HGUI_PANEL_TITLE: &str = "Florynx Panel";
pub const HGUI_SHELL_TITLE: &str = "Florynx HGUI";
