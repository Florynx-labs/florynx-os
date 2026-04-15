// =============================================================================
// Florynx Kernel — Text Editor Window
// =============================================================================
// Simple text editor with toolbar and multi-line editing
// =============================================================================

use alloc::vec::Vec;
use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::event::{Event, Key, Rect};
use crate::gui::theme;
use crate::gui::widgets::{Button, Panel};

const MAX_LINES: usize = 32;
const MAX_LINE_LEN: usize = 80;

pub struct TextEditor {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    lines: Vec<[u8; MAX_LINE_LEN]>,
    line_lens: Vec<usize>,
    cursor_line: usize,
    cursor_col: usize,
    toolbar: Panel,
    save_btn: Button,
    clear_btn: Button,
}

impl TextEditor {
    pub fn new(x: usize, y: usize, w: usize, h: usize) -> Self {
        let toolbar = Panel::new(x, y, w, 40, crate::gui::widgets::panel::PanelLayout::Horizontal);
        let save_btn = Button::new(x + 10, y + 8, 80, 24, "Save");
        let clear_btn = Button::new(x + 100, y + 8, 80, 24, "Clear");

        let mut lines = Vec::new();
        let mut line_lens = Vec::new();
        lines.push([0u8; MAX_LINE_LEN]);
        line_lens.push(0);

        TextEditor {
            x, y, w, h,
            lines,
            line_lens,
            cursor_line: 0,
            cursor_col: 0,
            toolbar,
            save_btn,
            clear_btn,
        }
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    pub fn draw(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;

        // Background
        renderer::draw_rounded_rect(fb, self.x, self.y, self.w, self.h, 6, Color::rgb(18, 20, 25));

        // Toolbar
        self.toolbar.draw(fb);
        self.save_btn.draw(fb);
        self.clear_btn.draw(fb);

        // Text area background
        let text_y = self.y + 45;
        let text_h = self.h - 50;
        renderer::draw_rect(fb, self.x + 5, text_y, self.w - 10, text_h, Color::rgb(12, 14, 18));

        // Line numbers background
        renderer::draw_rect(fb, self.x + 5, text_y, 30, text_h, Color::rgb(20, 22, 27));

        // Draw text lines
        let line_height = 12;
        for (i, line_len) in self.line_lens.iter().enumerate() {
            let ly = text_y + 5 + i * line_height;
            if ly + line_height > text_y + text_h { break; }

            // Line number
            let line_num = alloc::format!("{}", i + 1);
            renderer::draw_text(fb, &line_num, self.x + 10, ly, Color::rgb(80, 85, 95), 1);

            // Line text
            let line_text = core::str::from_utf8(&self.lines[i][..*line_len]).unwrap_or("");
            renderer::draw_text(fb, line_text, self.x + 40, ly, t.text, 1);

            // Cursor
            if i == self.cursor_line {
                let cursor_x = self.x + 40 + self.cursor_col * 8;
                renderer::draw_vline(fb, cursor_x, ly, 10, t.accent);
            }
        }
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        // Check toolbar buttons first
        if self.save_btn.handle_event(event) {
            crate::serial_println!("[text_editor] Save clicked!");
            return true;
        }
        if self.clear_btn.handle_event(event) {
            self.clear();
            return true;
        }

        // Handle keyboard input
        match *event {
            Event::KeyPress { key } => {
                match key {
                    Key::Char(c) if self.cursor_line < self.lines.len() => {
                        let line_len = self.line_lens[self.cursor_line];
                        if line_len < MAX_LINE_LEN {
                            // Insert character at cursor
                            if self.cursor_col < line_len {
                                // Shift text right
                                for i in (self.cursor_col..line_len).rev() {
                                    self.lines[self.cursor_line][i + 1] = self.lines[self.cursor_line][i];
                                }
                            }
                            self.lines[self.cursor_line][self.cursor_col] = c as u8;
                            self.line_lens[self.cursor_line] += 1;
                            self.cursor_col += 1;
                            return true;
                        }
                    }
                    Key::Backspace => {
                        if self.cursor_col > 0 {
                            // Delete character before cursor
                            let line_len = self.line_lens[self.cursor_line];
                            for i in self.cursor_col..line_len {
                                self.lines[self.cursor_line][i - 1] = self.lines[self.cursor_line][i];
                            }
                            self.line_lens[self.cursor_line] -= 1;
                            self.cursor_col -= 1;
                            return true;
                        } else if self.cursor_line > 0 {
                            // Join with previous line
                            let prev_len = self.line_lens[self.cursor_line - 1];
                            let curr_len = self.line_lens[self.cursor_line];
                            if prev_len + curr_len <= MAX_LINE_LEN {
                                // Copy current line to end of previous
                                for i in 0..curr_len {
                                    self.lines[self.cursor_line - 1][prev_len + i] = self.lines[self.cursor_line][i];
                                }
                                self.line_lens[self.cursor_line - 1] += curr_len;
                                // Remove current line
                                self.lines.remove(self.cursor_line);
                                self.line_lens.remove(self.cursor_line);
                                self.cursor_line -= 1;
                                self.cursor_col = prev_len;
                                return true;
                            }
                        }
                    }
                    Key::Enter if self.lines.len() < MAX_LINES => {
                        // Create new line
                        let curr_len = self.line_lens[self.cursor_line];
                        let mut new_line = [0u8; MAX_LINE_LEN];
                        let mut new_len = 0;

                        // Move text after cursor to new line
                        if self.cursor_col < curr_len {
                            for i in self.cursor_col..curr_len {
                                new_line[i - self.cursor_col] = self.lines[self.cursor_line][i];
                            }
                            new_len = curr_len - self.cursor_col;
                            self.line_lens[self.cursor_line] = self.cursor_col;
                        }

                        self.cursor_line += 1;
                        self.lines.insert(self.cursor_line, new_line);
                        self.line_lens.insert(self.cursor_line, new_len);
                        self.cursor_col = 0;
                        return true;
                    }
                    Key::ArrowLeft => {
                        if self.cursor_col > 0 {
                            self.cursor_col -= 1;
                        } else if self.cursor_line > 0 {
                            self.cursor_line -= 1;
                            self.cursor_col = self.line_lens[self.cursor_line];
                        }
                    }
                    Key::ArrowRight => {
                        if self.cursor_col < self.line_lens[self.cursor_line] {
                            self.cursor_col += 1;
                        } else if self.cursor_line < self.lines.len() - 1 {
                            self.cursor_line += 1;
                            self.cursor_col = 0;
                        }
                    }
                    Key::ArrowUp if self.cursor_line > 0 => {
                        self.cursor_line -= 1;
                        self.cursor_col = self.cursor_col.min(self.line_lens[self.cursor_line]);
                    }
                    Key::ArrowDown if self.cursor_line < self.lines.len() - 1 => {
                        self.cursor_line += 1;
                        self.cursor_col = self.cursor_col.min(self.line_lens[self.cursor_line]);
                    }
                    Key::Home => {
                        self.cursor_col = 0;
                    }
                    Key::End => {
                        self.cursor_col = self.line_lens[self.cursor_line];
                    }
                    _ => {}
                }
                false
            }
            _ => false,
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
        self.line_lens.clear();
        self.lines.push([0u8; MAX_LINE_LEN]);
        self.line_lens.push(0);
        self.cursor_line = 0;
        self.cursor_col = 0;
    }
}
