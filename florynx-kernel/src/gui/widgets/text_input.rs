// =============================================================================
// Florynx Kernel — TextInput Widget (macOS-style)
// =============================================================================
// Single-line text input field with:
//   - Cursor positioning via click
//   - Blinking cursor (PIT tick-based)
//   - Arrow key navigation
//   - Text selection (shift+arrow)
//   - Anti-aliased text rendering
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager, FontSize};
use crate::gui::event::{Event, Key, MouseButton, Rect};
use crate::gui::theme;

const MAX_TEXT: usize = 128;
/// Cursor blink period in PIT ticks (~200 ticks/sec → 100 ticks = 500ms).
const BLINK_PERIOD: u64 = 100;

pub struct TextInput {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    text: [u8; MAX_TEXT],
    text_len: usize,
    cursor_pos: usize,
    selection_start: Option<usize>,
    focused: bool,
    cursor_visible: bool,
    last_blink_tick: u64,
}

impl TextInput {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        TextInput {
            x, y, w, h,
            text: [0u8; MAX_TEXT],
            text_len: 0,
            cursor_pos: 0,
            selection_start: None,
            focused: false,
            cursor_visible: true,
            last_blink_tick: 0,
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
        self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
    }

    /// Compute cursor X position from text position (proportional).
    fn cursor_x_at(&self, pos: usize) -> usize {
        let text = core::str::from_utf8(&self.text[..pos.min(self.text_len)]).unwrap_or("");
        self.x + 8 + renderer::measure_text_aa(text, FontSize::Normal)
    }

    /// Compute text position from click X (proportional).
    fn pos_from_x(&self, click_x: usize) -> usize {
        let text_start = self.x + 8;
        if click_x <= text_start { return 0; }

        let mut accumulated = 0usize;
        for i in 0..self.text_len {
            let ch_adv = renderer::char_advance_aa(self.text[i] as char, FontSize::Normal);
            if text_start + accumulated + ch_adv / 2 >= click_x {
                return i;
            }
            accumulated += ch_adv;
        }
        self.text_len
    }

    /// Get selection range as (start, end) sorted.
    fn selection_range(&self) -> Option<(usize, usize)> {
        self.selection_start.map(|s| {
            let a = s.min(self.cursor_pos);
            let b = s.max(self.cursor_pos);
            (a, b)
        })
    }

    /// Delete selected text and collapse cursor.
    fn delete_selection(&mut self) -> bool {
        if let Some((start, end)) = self.selection_range() {
            if start != end {
                let remove_len = end - start;
                for i in end..self.text_len {
                    self.text[i - remove_len] = self.text[i];
                }
                self.text_len -= remove_len;
                self.cursor_pos = start;
                self.selection_start = None;
                return true;
            }
        }
        self.selection_start = None;
        false
    }

    pub fn draw(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;

        // Background
        let bg = if self.focused {
            Color::rgb(30, 35, 42)
        } else {
            Color::rgb(25, 28, 35)
        };
        renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 6, bg);

        // Border (accent glow when focused)
        let border = if self.focused {
            t.accent
        } else {
            Color::rgb(50, 55, 65)
        };
        renderer::draw_rounded_border(fb, self.x, self.y, self.w, self.h, 6, border);

        let text_y = self.y + (self.h.saturating_sub(8)) / 2;

        // Draw selection highlight
        if self.focused {
            if let Some((sel_start, sel_end)) = self.selection_range() {
                if sel_start != sel_end {
                    let x1 = self.cursor_x_at(sel_start);
                    let x2 = self.cursor_x_at(sel_end);
                    renderer::draw_rect(fb, x1, text_y.saturating_sub(1), x2 - x1, 10,
                        Color::rgba(41, 211, 208, 60));
                }
            }
        }

        // Text (AA)
        let text = self.get_text();
        renderer::draw_text_aa(fb, text, self.x + 8, text_y, t.text, FontSize::Normal);

        // Cursor (blinking, using PIT ticks for stable timing)
        if self.focused {
            let now = crate::drivers::timer::pit::get_ticks();
            if now.wrapping_sub(self.last_blink_tick) >= BLINK_PERIOD {
                self.cursor_visible = !self.cursor_visible;
                self.last_blink_tick = now;
            }

            if self.cursor_visible {
                let cursor_x = self.cursor_x_at(self.cursor_pos);
                renderer::draw_vline(fb, cursor_x, text_y.saturating_sub(1), 10, t.accent);
            }
        }
    }

    /// Handle event. Returns true if text changed.
    pub fn handle_event(&mut self, event: &Event) -> bool {
        match *event {
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                let was_focused = self.focused;
                self.focused = self.bounds().contains(x, y);
                if self.focused {
                    // Click-to-position cursor
                    self.cursor_pos = self.pos_from_x(x);
                    self.selection_start = None;
                    self.cursor_visible = true;
                    self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                }
                if self.focused != was_focused {
                    self.cursor_visible = self.focused;
                    self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                }
                false
            }
            Event::KeyPress { key } if self.focused => {
                match key {
                    Key::Char(c) => {
                        // Delete selection first if any
                        self.delete_selection();
                        if self.text_len < MAX_TEXT {
                            // Insert at cursor position
                            if self.cursor_pos < self.text_len {
                                for i in (self.cursor_pos..self.text_len).rev() {
                                    self.text[i + 1] = self.text[i];
                                }
                            }
                            self.text[self.cursor_pos] = c as u8;
                            self.text_len += 1;
                            self.cursor_pos += 1;
                            self.cursor_visible = true;
                            self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                            return true;
                        }
                    }
                    Key::Backspace => {
                        if self.delete_selection() {
                            return true;
                        }
                        if self.cursor_pos > 0 {
                            for i in self.cursor_pos..self.text_len {
                                self.text[i - 1] = self.text[i];
                            }
                            self.text_len -= 1;
                            self.cursor_pos -= 1;
                            self.cursor_visible = true;
                            self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                            return true;
                        }
                    }
                    Key::Delete => {
                        if self.delete_selection() {
                            return true;
                        }
                        if self.cursor_pos < self.text_len {
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
                            self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                        }
                        self.selection_start = None;
                    }
                    Key::ArrowRight => {
                        if self.cursor_pos < self.text_len {
                            self.cursor_pos += 1;
                            self.cursor_visible = true;
                            self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                        }
                        self.selection_start = None;
                    }
                    Key::Home => {
                        self.cursor_pos = 0;
                        self.cursor_visible = true;
                        self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                        self.selection_start = None;
                    }
                    Key::End => {
                        self.cursor_pos = self.text_len;
                        self.cursor_visible = true;
                        self.last_blink_tick = crate::drivers::timer::pit::get_ticks();
                        self.selection_start = None;
                    }
                    _ => {}
                }
                false
            }
            _ => false,
        }
    }
}
