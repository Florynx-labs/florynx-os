use alloc::string::String;

use crate::gui::ui::animation::tick_value;
use crate::gui::ui::event::{Event, EventResult};
use crate::gui::ui::geometry::{Constraints, Rect, Size};
use crate::gui::ui::render_context::RenderContext;
use crate::gui::ui::widget::{BaseWidgetState, Widget, WidgetId};

use super::base::base_state;

pub struct Input {
    base: BaseWidgetState,
    pub text: String,
    placeholder: String,
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
        }
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
        if self.text.is_empty() {
            ctx.draw_text(self.base.rect.x + 8, self.base.rect.y + 9, &self.placeholder);
        } else {
            ctx.draw_text(self.base.rect.x + 8, self.base.rect.y + 9, &self.text);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match *event {
            Event::Click(x, y) => {
                let hit = self.base.rect.contains(crate::gui::ui::geometry::Point { x, y });
                if hit {
                    return EventResult::HandledAndFocus(self.base.id);
                }
                EventResult::Ignored
            }
            Event::KeyPress(ch) => {
                if self.base.focused {
                    if ch == '\u{8}' {
                        self.text.pop();
                    } else {
                        self.text.push(ch);
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
