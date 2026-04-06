// =============================================================================
// Florynx Kernel — GUI Event System
// =============================================================================
// Input events and geometry helpers for the GUI component system.
// =============================================================================

// ---------------------------------------------------------------------------
// Geometry
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

impl Rect {
    pub const fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        Rect { x, y, w, h }
    }

    pub fn contains(&self, px: usize, py: usize) -> bool {
        px >= self.x && px < self.x + self.w && py >= self.y && py < self.y + self.h
    }
}

// ---------------------------------------------------------------------------
// Mouse events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    MouseMove  { x: usize, y: usize },
    MouseDown  { x: usize, y: usize, button: MouseButton },
    MouseUp    { x: usize, y: usize, button: MouseButton },
}

impl Event {
    pub fn position(&self) -> (usize, usize) {
        match *self {
            Event::MouseMove { x, y } => (x, y),
            Event::MouseDown { x, y, .. } => (x, y),
            Event::MouseUp   { x, y, .. } => (x, y),
        }
    }
}
