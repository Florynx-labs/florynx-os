use super::event::{Event, EventResult};
use super::geometry::{Constraints, Rect, Size};
use super::render_context::RenderContext;

pub type WidgetId = u64;

#[derive(Debug, Clone, Copy)]
pub struct BaseWidgetState {
    pub id: WidgetId,
    pub rect: Rect,
    pub desired: Size,
    pub dirty: bool,
    pub focusable: bool,
    pub hovered: bool,
    pub pressed: bool,
    pub focused: bool,
    pub hover_t: f32,
    pub press_t: f32,
    pub scale_t: f32,
}

impl BaseWidgetState {
    pub fn new(id: WidgetId) -> Self {
        Self {
            id,
            rect: Rect::new(0, 0, 0, 0),
            desired: Size { w: 120, h: 32 },
            dirty: true,
            focusable: false,
            hovered: false,
            pressed: false,
            focused: false,
            hover_t: 0.0,
            press_t: 0.0,
            scale_t: 0.0,
        }
    }
}

pub trait Widget {
    fn base(&self) -> &BaseWidgetState;
    fn base_mut(&mut self) -> &mut BaseWidgetState;
    fn layout(&mut self, constraints: Constraints) -> Size;
    fn set_position(&mut self, x: i32, y: i32);
    fn render(&self, ctx: &mut RenderContext<'_>);
    fn handle_event(&mut self, event: &Event) -> EventResult;
    fn tick_animations(&mut self, dt_ms: u32);
    fn for_each_child_mut(&mut self, _f: &mut dyn FnMut(&mut dyn Widget)) {}
}
