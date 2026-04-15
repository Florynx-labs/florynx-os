use alloc::string::String;

use crate::gui::ui::animation::tick_value;
use crate::gui::ui::event::{Event, EventResult};
use crate::gui::ui::geometry::{Constraints, Rect, Size};
use crate::gui::ui::render_context::RenderContext;
use crate::gui::ui::widget::{BaseWidgetState, Widget, WidgetId};

use super::base::base_state;

pub struct Button {
    base: BaseWidgetState,
    pub label: String,
    pub on_click: Option<fn()>,
}

impl Button {
    pub fn new(id: WidgetId, label: &str) -> Self {
        let mut base = base_state(id);
        base.desired = Size {
            w: (label.len() as i32 * 10 + 22).max(88),
            h: 36,
        };
        base.focusable = true;
        Self {
            base,
            label: String::from(label),
            on_click: None,
        }
    }

    fn bg_color(&self) -> u32 {
        let normal = 0x2A2F3A as f32;
        let hover = 0x394250 as f32;
        let press = 0x566175 as f32;
        let mut color = normal + (hover - normal) * self.base.hover_t;
        color = color + (press - color) * self.base.press_t;
        color as u32
    }
}

impl Widget for Button {
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
        ctx.draw_rect(self.base.rect, self.bg_color());
        ctx.draw_text(self.base.rect.x + 8, self.base.rect.y + 10, &self.label);
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        match *event {
            Event::MouseMove(x, y) => {
                self.base.hovered = self.base.rect.contains(crate::gui::ui::geometry::Point { x, y });
                EventResult::Ignored
            }
            Event::Click(x, y) => {
                let hit = self.base.rect.contains(crate::gui::ui::geometry::Point { x, y });
                self.base.pressed = hit;
                if hit {
                    if let Some(cb) = self.on_click {
                        cb();
                    }
                    return EventResult::HandledAndFocus(self.base.id);
                }
                EventResult::Ignored
            }
            Event::KeyPress(_) => EventResult::Ignored,
        }
    }

    fn tick_animations(&mut self, dt_ms: u32) {
        let target_hover = if self.base.hovered { 1.0 } else { 0.0 };
        let target_press = if self.base.pressed { 1.0 } else { 0.0 };
        self.base.hover_t = tick_value(self.base.hover_t, target_hover, dt_ms, 12.0);
        self.base.press_t = tick_value(self.base.press_t, target_press, dt_ms, 18.0);
        self.base.pressed = false;
    }
}
