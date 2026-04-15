use alloc::boxed::Box;

use crate::gui::api::GuiEventV1;

use super::event::{Event, EventResult};
use super::geometry::{Constraints, Rect, Size};
use super::render_context::{RenderBackend, RenderContext, SyscallRenderBackend};
use super::widget::{Widget, WidgetId};

pub struct UiRuntime {
    pub root: Box<dyn Widget>,
    pub window_id: u32,
    pub screen_size: Size,
    pub focused_widget: Option<WidgetId>,
    pub cursor_raw: (f32, f32),
    pub cursor_smoothed: (f32, f32),
}

impl UiRuntime {
    pub fn new(window_id: u32, root: Box<dyn Widget>, screen_size: Size) -> Self {
        Self {
            root,
            window_id,
            screen_size,
            focused_widget: None,
            cursor_raw: (0.0, 0.0),
            cursor_smoothed: (0.0, 0.0),
        }
    }

    pub fn layout(&mut self) {
        let constraints = Constraints::tight(self.screen_size);
        let _ = self.root.layout(constraints);
        self.root.set_position(0, 0);
    }

    pub fn tick_animations(&mut self, dt_ms: u32) {
        self.root.tick_animations(dt_ms);
        let alpha = ((dt_ms as f32) / 120.0).clamp(0.05, 0.5);
        self.cursor_smoothed.0 += (self.cursor_raw.0 - self.cursor_smoothed.0) * alpha;
        self.cursor_smoothed.1 += (self.cursor_raw.1 - self.cursor_smoothed.1) * alpha;
    }

    pub fn handle_event(&mut self, event: Event) -> EventResult {
        if let Some((x, y)) = match event {
            Event::MouseMove(x, y) => Some((x, y)),
            Event::Click(x, y) => Some((x, y)),
            Event::KeyPress(_) => None,
        } {
            self.cursor_raw = (x as f32, y as f32);
        }

        let consumed = self.root.handle_event(&event);
        match consumed {
            EventResult::HandledAndFocus(id) => {
                self.focused_widget = Some(id);
                self.apply_focus(id);
                EventResult::Handled
            }
            _ => consumed,
        }
    }

    fn apply_focus(&mut self, target: WidgetId) {
        fn walk_focus(node: &mut dyn Widget, target: WidgetId) {
            let base = node.base_mut();
            base.focused = base.id == target;
            let mut recurse = |child: &mut dyn Widget| walk_focus(child, target);
            node.for_each_child_mut(&mut recurse);
        }
        walk_focus(self.root.as_mut(), target);
    }

    pub fn render(&mut self) {
        let mut backend = SyscallRenderBackend::new(self.window_id);
        self.render_with_backend(&mut backend);
    }

    pub fn render_with_backend(&mut self, backend: &mut dyn RenderBackend) {
        let mut ctx = RenderContext::new(backend);
        self.root.render(&mut ctx);
        backend.submit();
    }

    pub fn map_kernel_event(ev: GuiEventV1) -> Option<Event> {
        match ev {
            GuiEventV1::MouseState { x, y, buttons, .. } => {
                if (buttons & 1) != 0 {
                    Some(Event::Click(x as i32, y as i32))
                } else {
                    Some(Event::MouseMove(x as i32, y as i32))
                }
            }
            GuiEventV1::KeyPress { code, .. } => {
                let ch = match code {
                    8 => '\u{8}',
                    32..=126 => code as u8 as char,
                    _ => return None,
                };
                Some(Event::KeyPress(ch))
            }
            GuiEventV1::WindowCreated { .. } | GuiEventV1::WindowDestroyed { .. } => None,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(0, 0, self.screen_size.w, self.screen_size.h)
    }
}
