// =============================================================================
// Florynx Kernel — Panel Widget
// =============================================================================
// Container widget for organizing UI elements with layout management
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::event::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelLayout {
    Vertical,
    Horizontal,
}

pub struct Panel {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub layout: PanelLayout,
    pub padding: usize,
    pub spacing: usize,
    pub background: Color,
    pub border: Option<Color>,
}

impl Panel {
    pub fn new(x: usize, y: usize, w: usize, h: usize, layout: PanelLayout) -> Self {
        Panel {
            x, y, w, h,
            layout,
            padding: 8,
            spacing: 4,
            background: Color::rgb(20, 23, 28),
            border: Some(Color::rgb(50, 55, 65)),
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    pub fn draw(&self, fb: &mut FramebufferManager) {
        // Background
        renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 4, self.background);

        // Border
        if let Some(border_color) = self.border {
            renderer::draw_rounded_border(fb, self.x, self.y, self.w, self.h, 4, border_color);
        }
    }

    /// Calculate the position for the next child widget based on layout.
    /// Returns (x, y) for the next widget given the index and previous widget dimensions.
    pub fn child_position(&self, index: usize, prev_w: usize, prev_h: usize) -> (usize, usize) {
        match self.layout {
            PanelLayout::Vertical => {
                let x = self.x + self.padding;
                let y = if index == 0 {
                    self.y + self.padding
                } else {
                    self.y + self.padding + index * (prev_h + self.spacing)
                };
                (x, y)
            }
            PanelLayout::Horizontal => {
                let x = if index == 0 {
                    self.x + self.padding
                } else {
                    self.x + self.padding + index * (prev_w + self.spacing)
                };
                let y = self.y + self.padding;
                (x, y)
            }
        }
    }

    /// Get the available content width (excluding padding).
    pub fn content_width(&self) -> usize {
        self.w.saturating_sub(self.padding * 2)
    }

    /// Get the available content height (excluding padding).
    pub fn content_height(&self) -> usize {
        self.h.saturating_sub(self.padding * 2)
    }
}
