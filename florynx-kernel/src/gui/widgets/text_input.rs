// =============================================================================
// Florynx Kernel — TextInput Widget
// =============================================================================
// Single-line text input field with cursor and selection
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::event::{Event, Key, MouseButton, Rect};
use crate::gui::theme;

const MAX_TEXT: usize = 128;

pub struct TextInput {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    text: [u8; MAX_TEXT],
    text_len: usize,
    cursor_pos: usize,
    focused: bool,
    cursor_visible: bool,
    cursor_blink_counter: u32,
}

impl TextInput {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        TextInput {
            x, y, w, h,
            text: [0u8; MAX_TEXT],
            text_len: 0,
            cursor_pos: 0,
            focused: false,
            cursor_visible: true,
            cursor_blink_counter: 0,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    pub fn set_text(&mut self, text: &str) {
        let len = text.len().min(MAX_TEXT);
        self.text[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.text_len = len;
        self.cursor_pos = len;
    }

    pub fn get_text(&self) -> &str {
        core::str::from_utf8(&self.text[..self.text_len]).unwrap_or("")
    }

    pub fn set_focused(&mut self, focused: bool) {
        self.focused = focused;
        self.cursor_visible = focused;
        self.cursor_blink_counter = 0;
    }

    pub fn draw(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;
        
        // Background
        let bg = if self.focused {
            Color::rgb(30, 35, 42)
        } else {
            Color::rgb(25, 28, 35)
        };
        renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 3, bg);

        // Border
        let border = if self.focused {
            t.accent
        } else {
            Color::rgb(50, 55, 65)
        };
        renderer::draw_rounded_border(fb, self.x, self.y, self.w, self.h, 3, border);

        // Text
        let text = self.get_text();
        let text_x = self.x + 8;
        let text_y = self.y + (self.h.saturating_sub(8)) / 2;
        renderer::draw_text(fb, text, text_x, text_y, t.text, 1);

        // Cursor (blinking)
        if self.focused {
            self.cursor_blink_counter += 1;
            if self.cursor_blink_counter > 30 {
                self.cursor_visible = !self.cursor_visible;
                self.cursor_blink_counter = 0;
            }

            if self.cursor_visible {
                let cursor_x = text_x + self.cursor_pos * 8;
                let cursor_y = text_y;
                renderer::draw_vline(fb, cursor_x, cursor_y, 8, t.accent);
            }
        }
    }

    /// Handle event. Returns true if text changed.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        match *event {
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                let was_focused = self.focused;
                self.focused = self.bounds().contains(x, y);
                if self.focused != was_focused {
                    self.cursor_visible = self.focused;
                    self.cursor_blink_counter = 0;
                }
                false
            }
            Event::KeyPress { key } if self.focused => {
                match key {
                    Key::Char(c) => {
                        if self.text_len < MAX_TEXT {
                            // Insert at cursor position
                            if self.cursor_pos < self.text_len {
                                // Shift text right
                                for i in (self.cursor_pos..self.text_len).rev() {
                                    self.text[i + 1] = self.text[i];
                                }
                            }
                            self.text[self.cursor_pos] = c as u8;
                            self.text_len += 1;
                            self.cursor_pos += 1;
                            self.cursor_visible = true;
                            self.cursor_blink_counter = 0;
                            return true;
                        }
                    }
                    Key::Backspace => {
                        if self.cursor_pos > 0 {
                            // Shift text left
                            for i in self.cursor_pos..self.text_len {
                                self.text[i - 1] = self.text[i];
                            }
                            self.text_len -= 1;
                            self.cursor_pos -= 1;
                            self.cursor_visible = true;
                            self.cursor_blink_counter = 0;
                            return true;
                        }
                    }
                    Key::Delete => {
                        if self.cursor_pos < self.text_len {
                            // Shift text left
                            for i in (self.cursor_pos + 1)..self.text_len {
                                self.text[i - 1] = self.text[i];
                            }
                            self.text_len -= 1;
                            return true;
                        }
                    }
                    Key::ArrowLeft => {
                        if self.cursor_pos > 0 {
                            self.cursor_pos -= 1;
                            self.cursor_visible = true;
                            self.cursor_blink_counter = 0;
                        }
                    }
                    Key::ArrowRight => {
                        if self.cursor_pos < self.text_len {
                            self.cursor_pos += 1;
                            self.cursor_visible = true;
                            self.cursor_blink_counter = 0;
                        }
                    }
                    Key::Home => {
                        self.cursor_pos = 0;
                        self.cursor_visible = true;
                        self.cursor_blink_counter = 0;
                    }
                    Key::End => {
                        self.cursor_pos = self.text_len;
                        self.cursor_visible = true;
                        self.cursor_blink_counter = 0;
                    }
                    _ => {}
                }
                false
            }
            _ => false,
        }
    }
}
