use alloc::string::String;
use alloc::vec::Vec;

use crate::gui::api;

use super::geometry::Rect;

pub trait RenderBackend {
    fn draw_rect(&mut self, rect: Rect, color: u32);
    fn draw_text(&mut self, x: i32, y: i32, text: &str);
    fn submit(&mut self);
}

#[derive(Debug, Clone)]
pub enum DrawOp {
    Rect { rect: Rect, color: u32 },
    Text { x: i32, y: i32, text: String },
}

pub struct RenderContext<'a> {
    backend: &'a mut dyn RenderBackend,
}

impl<'a> RenderContext<'a> {
    pub fn new(backend: &'a mut dyn RenderBackend) -> Self {
        Self { backend }
    }

    pub fn draw_rect(&mut self, rect: Rect, color: u32) {
        self.backend.draw_rect(rect, color);
    }

    pub fn draw_text(&mut self, x: i32, y: i32, text: &str) {
        self.backend.draw_text(x, y, text);
    }
}

pub struct SyscallRenderBackend {
    pub win_id: u32,
    pub ops: Vec<DrawOp>,
    pub dirty: Option<Rect>,
}

impl SyscallRenderBackend {
    pub fn new(win_id: u32) -> Self {
        Self {
            win_id,
            ops: Vec::new(),
            dirty: None,
        }
    }

    fn mark_dirty(&mut self, rect: Rect) {
        self.dirty = Some(match self.dirty {
            Some(old) => old.union(rect),
            None => rect,
        });
    }
}

impl RenderBackend for SyscallRenderBackend {
    fn draw_rect(&mut self, rect: Rect, color: u32) {
        self.mark_dirty(rect);
        self.ops.push(DrawOp::Rect { rect, color });
    }

    fn draw_text(&mut self, x: i32, y: i32, text: &str) {
        let width = (text.len() as i32).saturating_mul(8).max(8);
        self.mark_dirty(Rect { x, y, w: width, h: 18 });
        self.ops.push(DrawOp::Text {
            x,
            y,
            text: String::from(text),
        });
    }

    fn submit(&mut self) {
        for op in self.ops.drain(..) {
            match op {
                DrawOp::Rect { rect, color } => {
                    let _ = api::draw_rect(
                        self.win_id,
                        rect.x.max(0) as u32,
                        rect.y.max(0) as u32,
                        rect.w.max(0) as u32,
                        rect.h.max(0) as u32,
                        color,
                    );
                }
                DrawOp::Text { x: _, y: _, text } => {
                    // Current syscall ABI draws text at compositor-managed default origin.
                    let _ = api::draw_text(self.win_id, &text);
                }
            }
        }
        if self.dirty.is_some() {
            let _ = api::invalidate(self.win_id);
        }
        self.dirty = None;
    }
}
