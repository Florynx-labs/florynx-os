use alloc::string::String;

use crate::gui::ui::animation::tick_value;
use crate::gui::ui::event::{Event, EventResult, KeyCode};
use crate::gui::ui::geometry::{Constraints, Rect, Size};
use crate::gui::ui::render_context::RenderContext;
use crate::gui::ui::widget::{BaseWidgetState, Widget, WidgetId};

use super::base::base_state;

pub struct Input {
    base: BaseWidgetState,
    pub text: String,
    placeholder: String,
    cursor_pos: usize,
    selection_start: Option<usize>,
}

impl Input {
    pub fn new(id: WidgetId, placeholder: &str) -> Self {
        let mut base = base_state(id);
        base.desired = Size { w: 220, h: 34 };
        base.focusable = true;
        Self {
            base,
            text: String::new(),
            placeholder: String::from(placeholder),
            cursor_pos: 0,
            selection_start: None,
        }
    }

    /// Insert a character at cursor position.
    fn insert_char(&mut self, c: char) {
        if self.cursor_pos >= self.text.len() {
            self.text.push(c);
        } else {
            self.text.insert(self.cursor_pos, c);
        }
        self.cursor_pos += 1;
        self.selection_start = None;
    }

    /// Delete character before cursor (backspace).
    fn backspace(&mut self) -> bool {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            if self.cursor_pos < self.text.len() {
                self.text.remove(self.cursor_pos);
            }
            self.selection_start = None;
            return true;
        }
        false
    }

    /// Delete character at cursor (forward delete).
    fn delete_forward(&mut self) -> bool {
        if self.cursor_pos < self.text.len() {
            self.text.remove(self.cursor_pos);
            self.selection_start = None;
            return true;
        }
        false
    }
}

impl Widget for Input {
    fn base(&self) -> &BaseWidgetState {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseWidgetState {
        &mut self.base
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let size = constraints.clamp(self.base.desired);
        self.base.rect.w = size.w;
        self.base.rect.h = size.h;
        size
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.base.rect = Rect::new(x, y, self.base.rect.w, self.base.rect.h);
    }

    fn render(&self, ctx: &mut RenderContext<'_>) {
        let bg = if self.base.focused { 0x263342 } else { 0x1E232B };
        ctx.draw_rect(self.base.rect, bg);

        // Border glow when focused
        if self.base.focused {
            let r = self.base.rect;
            // Top border
            ctx.draw_rect(Rect::new(r.x, r.y, r.w, 1), 0x29D3D0);
            // Bottom border
            ctx.draw_rect(Rect::new(r.x, r.y + r.h - 1, r.w, 1), 0x29D3D0);
            // Left border
            ctx.draw_rect(Rect::new(r.x, r.y, 1, r.h), 0x29D3D0);
            // Right border
            ctx.draw_rect(Rect::new(r.x + r.w - 1, r.y, 1, r.h), 0x29D3D0);
        }

        if self.text.is_empty() {
            ctx.draw_text(self.base.rect.x + 8, self.base.rect.y + 9, &self.placeholder);
        } else {
            ctx.draw_text(self.base.rect.x + 8, self.base.rect.y + 9, &self.text);
        }

        // Draw cursor indicator (simple line at cursor position)
        if self.base.focused && !self.text.is_empty() {
            let cursor_x = self.base.rect.x + 8 + (self.cursor_pos as i32) * 7;
            ctx.draw_rect(Rect::new(cursor_x, self.base.rect.y + 7, 1, 16), 0x29D3D0);
        } else if self.base.focused && self.text.is_empty() {
            ctx.draw_rect(Rect::new(self.base.rect.x + 8, self.base.rect.y + 7, 1, 16), 0x29D3D0);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match *event {
            Event::Click(x, y) => {
                let hit = self.base.rect.contains(crate::gui::ui::geometry::Point { x, y });
                if hit {
                    // Click-to-position cursor
                    let offset = (x - self.base.rect.x - 8).max(0);
                    self.cursor_pos = ((offset / 7) as usize).min(self.text.len());
                    self.selection_start = None;
                    return EventResult::HandledAndFocus(self.base.id);
                }
                EventResult::Ignored
            }
            Event::KeyDown(key) => {
                if self.base.focused {
                    match key {
                        KeyCode::Char(ch) => {
                            self.insert_char(ch);
                            return EventResult::Handled;
                        }
                        KeyCode::Backspace => {
                            if self.backspace() {
                                return EventResult::Handled;
                            }
                        }
                        KeyCode::Delete => {
                            if self.delete_forward() {
                                return EventResult::Handled;
                            }
                        }
                        KeyCode::ArrowLeft => {
                            if self.cursor_pos > 0 {
                                self.cursor_pos -= 1;
                                self.selection_start = None;
                            }
                            return EventResult::Handled;
                        }
                        KeyCode::ArrowRight => {
                            if self.cursor_pos < self.text.len() {
                                self.cursor_pos += 1;
                                self.selection_start = None;
                            }
                            return EventResult::Handled;
                        }
                        KeyCode::Home => {
                            self.cursor_pos = 0;
                            self.selection_start = None;
                            return EventResult::Handled;
                        }
                        KeyCode::End => {
                            self.cursor_pos = self.text.len();
                            self.selection_start = None;
                            return EventResult::Handled;
                        }
                        _ => {}
                    }
                    return EventResult::Ignored;
                }
                EventResult::Ignored
            }
            Event::KeyPress(ch) => {
                if self.base.focused {
                    if ch == '\u{8}' {
                        self.backspace();
                    } else {
                        self.insert_char(ch);
                    }
                    return EventResult::Handled;
                }
                EventResult::Ignored
            }
            Event::MouseMove(_, _) => EventResult::Ignored,
        }
    }

    fn tick_animations(&mut self, dt_ms: u32) {
        let target = if self.base.focused { 1.0 } else { 0.0 };
        self.base.scale_t = tick_value(self.base.scale_t, target, dt_ms, 14.0);
    }
}
