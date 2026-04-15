use super::geometry::Point;
use super::widget::WidgetId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Backspace,
    Delete,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    ArrowDown,
    Home,
    End,
    Enter,
    Tab,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    MouseMove(i32, i32),
    Click(i32, i32),
    KeyPress(char),
    /// Extended key event with full key code support.
    KeyDown(KeyCode),
}

impl Event {
    pub fn point(self) -> Option<Point> {
        match self {
            Event::MouseMove(x, y) | Event::Click(x, y) => Some(Point { x, y }),
            Event::KeyPress(_) | Event::KeyDown(_) => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventResult {
    Ignored,
    Handled,
    HandledAndFocus(WidgetId),
}
