// =============================================================================
// Florynx Kernel — Button Widget
// =============================================================================
// Clickable button with hover and pressed states
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    Normal,
    Hover,
    Pressed,
}

pub struct Button {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    label: [u8; 32],
    label_len: usize,
    state: ButtonState,
    pub enabled: bool,
}

impl Button {
    pub fn new(x: usize, y: usize, w: usize, h: usize, label: &str) -> Self {
        let mut lbl = [0u8; 32];
        let len = label.len().min(32);
        lbl[..len].copy_from_slice(&label.as_bytes()[..len]);
        Button {
            x, y, w, h,
            label: lbl,
            label_len: len,
            state: ButtonState::Normal,
            enabled: true,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    pub fn draw(&self, fb: &mut FramebufferManager) {
        if !self.enabled {
            // Disabled state - gray
            renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 4,
                Color::rgb(60, 60, 65));
            let text_color = Color::rgb(100, 100, 105);
            self.draw_label(fb, text_color);
            return;
        }

        let t = &theme::DARK;
        let bg = match self.state {
            ButtonState::Normal => Color::rgb(45, 50, 60),
            ButtonState::Hover => Color::rgb(55, 62, 75),
            ButtonState::Pressed => Color::rgb(35, 40, 50),
        };

        // Button background
        renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 4, bg);
        
        // Border
        let border = if self.state == ButtonState::Pressed {
            t.accent
        } else {
            Color::rgb(70, 75, 85)
        };
        renderer::draw_rounded_border(fb, self.x, self.y, self.w, self.h, 4, border);

        // Label
        let text_color = if self.state == ButtonState::Pressed {
            t.accent
        } else {
            t.text
        };
        self.draw_label(fb, text_color);
    }

    fn draw_label(&self, fb: &mut FramebufferManager, color: Color) {
        let label = core::str::from_utf8(&self.label[..self.label_len]).unwrap_or("");
        let text_w = label.len() * 8;
        let text_x = self.x + (self.w.saturating_sub(text_w)) / 2;
        let text_y = self.y + (self.h.saturating_sub(8)) / 2;
        renderer::draw_text(fb, label, text_x, text_y, color, 1);
    }

    /// Handle event. Returns true if button was clicked.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        if !self.enabled { return false; }

        match *event {
            Event::MouseMove { x, y } => {
                if self.bounds().contains(x, y) {
                    if self.state != ButtonState::Pressed {
                        self.state = ButtonState::Hover;
                    }
                } else {
                    if self.state != ButtonState::Pressed {
                        self.state = ButtonState::Normal;
                    }
                }
                false
            }
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                if self.bounds().contains(x, y) {
                    self.state = ButtonState::Pressed;
                    return false; // Wait for mouse up
                }
                false
            }
            Event::MouseUp { x, y, button: MouseButton::Left } => {
                if self.state == ButtonState::Pressed {
                    self.state = if self.bounds().contains(x, y) {
                        ButtonState::Hover
                    } else {
                        ButtonState::Normal
                    };
                    // Return true if mouse up happened inside button bounds
                    return self.bounds().contains(x, y);
                }
                false
            }
            _ => false,
        }
    }
}
