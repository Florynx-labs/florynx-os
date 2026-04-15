use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::gui::ui::event::{Event, EventResult};
use crate::gui::ui::geometry::{Constraints, Rect, Size};
use crate::gui::ui::layout::{layout_linear, Axis};
use crate::gui::ui::render_context::RenderContext;
use crate::gui::ui::widget::{BaseWidgetState, Widget, WidgetId};

use super::base::base_state;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Row,
    Column,
}

pub struct Container {
    base: BaseWidgetState,
    kind: ContainerKind,
    pub gap: i32,
    pub children: Vec<Box<dyn Widget>>,
}

impl Container {
    pub fn new(id: WidgetId, kind: ContainerKind) -> Self {
        let mut base = base_state(id);
        base.desired = Size { w: 320, h: 200 };
        Self {
            base,
            kind,
            gap: 8,
            children: Vec::new(),
        }
    }

    pub fn push(&mut self, child: Box<dyn Widget>) {
        self.children.push(child);
    }
}

impl Widget for Container {
    fn base(&self) -> &BaseWidgetState {
        &self.base
    }

    fn base_mut(&mut self) -> &mut BaseWidgetState {
        &mut self.base
    }

    fn layout(&mut self, constraints: Constraints) -> Size {
        let axis = match self.kind {
            ContainerKind::Row => Axis::Horizontal,
            ContainerKind::Column => Axis::Vertical,
        };
        let size = layout_linear(axis, &mut self.children, constraints.loosen(), self.gap);
        self.base.rect.w = size.w;
        self.base.rect.h = size.h;
        size
    }

    fn set_position(&mut self, x: i32, y: i32) {
        self.base.rect = Rect::new(x, y, self.base.rect.w, self.base.rect.h);
        for child in self.children.iter_mut() {
            let child_rect = child.base().rect;
            child.set_position(x + child_rect.x, y + child_rect.y);
        }
    }

    fn render(&self, ctx: &mut RenderContext<'_>) {
        for child in self.children.iter() {
            child.render(ctx);
        }
    }

    fn handle_event(&mut self, event: &Event) -> EventResult {
        for child in self.children.iter_mut().rev() {
            match child.handle_event(event) {
                EventResult::Ignored => {}
                consumed => return consumed,
            }
        }
        EventResult::Ignored
    }

    fn tick_animations(&mut self, dt_ms: u32) {
        for child in self.children.iter_mut() {
            child.tick_animations(dt_ms);
        }
    }

    fn for_each_child_mut(&mut self, f: &mut dyn FnMut(&mut dyn Widget)) {
        for child in self.children.iter_mut() {
            f(child.as_mut());
        }
    }
}
