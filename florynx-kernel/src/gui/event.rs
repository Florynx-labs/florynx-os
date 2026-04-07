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

    /// Check if two rectangles overlap.
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.w && self.x + self.w > other.x
            && self.y < other.y + other.h && self.y + self.h > other.y
    }

    /// Return the bounding box that contains both rectangles.
    pub fn union(&self, other: &Rect) -> Rect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.w).max(other.x + other.w);
        let y2 = (self.y + self.h).max(other.y + other.h);
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }

    /// Clamp rect to fit within screen dimensions.
    pub fn clamp(&self, sw: usize, sh: usize) -> Rect {
        let x = self.x.min(sw);
        let y = self.y.min(sh);
        let w = self.w.min(sw.saturating_sub(x));
        let h = self.h.min(sh.saturating_sub(y));
        Rect::new(x, y, w, h)
    }
}

// ---------------------------------------------------------------------------
// Input events
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Char(char),
    Backspace,
    Enter,
    Tab,
    Escape,
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    MouseMove  { x: usize, y: usize },
    MouseDown  { x: usize, y: usize, button: MouseButton },
    MouseUp    { x: usize, y: usize, button: MouseButton },
    KeyPress   { key: Key },
    KeyRelease { key: Key },
}

impl Event {
    pub fn position(&self) -> Option<(usize, usize)> {
        match *self {
            Event::MouseMove { x, y } => Some((x, y)),
            Event::MouseDown { x, y, .. } => Some((x, y)),
            Event::MouseUp   { x, y, .. } => Some((x, y)),
            _ => None,
        }
    }
}
