use alloc::string::String;

use crate::gui::ui::animation::tick_value;
use crate::gui::ui::event::{Event, EventResult};
use crate::gui::ui::geometry::{Constraints, Rect, Size};
use crate::gui::ui::render_context::RenderContext;
use crate::gui::ui::widget::{BaseWidgetState, Widget, WidgetId};

use super::base::base_state;

pub struct Text {
    base: BaseWidgetState,
    pub content: String,
    pub color: u32,
}

impl Text {
    pub fn new(id: WidgetId, content: &str) -> Self {
        let mut base = base_state(id);
        base.desired = Size {
            w: (content.len() as i32 * 8).max(24),
            h: 20,
        };
        Self {
            base,
            content: String::from(content),
            color: 0xD6DEEB,
        }
    }
}

impl Widget for Text {
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
        ctx.draw_text(self.base.rect.x, self.base.rect.y, &self.content);
    }

    fn handle_event(&mut self, _event: &Event) -> EventResult {
        EventResult::Ignored
    }

    fn tick_animations(&mut self, dt_ms: u32) {
        self.base.hover_t = tick_value(self.base.hover_t, 0.0, dt_ms, 14.0);
    }
}
