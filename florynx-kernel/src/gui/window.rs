// =============================================================================
// Florynx Kernel — GUI Window Component
// =============================================================================
// Draggable window with rounded titlebar, traffic-light buttons, and content.
// =============================================================================

use alloc::vec::Vec;
use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::theme;
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::animation::{AnimatedPos, AnimatedOpacity};

const MAX_TITLE: usize = 48;
const MAX_CONTENT: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowId {
    Id(usize),
}

/// Window drag animation speed (0.0 = frozen, 1.0 = instant).
const DRAG_SPEED: f32 = 0.35;
/// Window open fade-in speed.
const OPEN_SPEED: f32 = 0.12;

pub struct Window {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    title: [u8; MAX_TITLE],
    title_len: usize,
    content: [u8; MAX_CONTENT],
    content_len: usize,
    pub active: bool,
    pub visible: bool,
    // drag state
    dragging: bool,
    drag_ox: usize,
    drag_oy: usize,
    // animations
    pub anim_pos: AnimatedPos,
    pub anim_opacity: AnimatedOpacity,
    // per-window offscreen buffer (RGB, 3 bytes per pixel)
    // Includes shadow area: total_w × total_h
    pub buffer: Vec<u8>,
    pub buf_w: usize,
    pub buf_h: usize,
    /// True if the window content changed and needs to be redrawn to its buffer.
    pub dirty: bool,
}

impl Window {
    pub fn new(id: usize, x: usize, y: usize, w: usize, h: usize, title: &str) -> Self {
        let mut t = [0u8; MAX_TITLE];
        let tlen = title.len().min(MAX_TITLE);
        t[..tlen].copy_from_slice(&title.as_bytes()[..tlen]);
        let th = &theme::DARK;
        let extra = th.shadow_layers * 2 + 2;
        let buf_w = w + extra;
        let buf_h = h + extra;
        Window {
            id, x, y, w, h,
            title: t,
            title_len: tlen,
            content: [0u8; MAX_CONTENT],
            content_len: 0,
            active: false,
            visible: true,
            dragging: false,
            drag_ox: 0,
            drag_oy: 0,
            anim_pos: AnimatedPos::new(x as f32, y as f32, DRAG_SPEED),
            anim_opacity: AnimatedOpacity::new(0.0, OPEN_SPEED),
            buffer: Vec::new(), // allocated lazily on first render
            buf_w,
            buf_h,
            dirty: true, // needs initial draw
        }
    }

    pub fn set_content(&mut self, text: &str) {
        let len = text.len().min(MAX_CONTENT);
        self.content[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.content_len = len;
        self.dirty = true;
    }

    pub fn title_str(&self) -> &str {
        core::str::from_utf8(&self.title[..self.title_len]).unwrap_or("?")
    }

    pub fn content_str(&self) -> &str {
        core::str::from_utf8(&self.content[..self.content_len]).unwrap_or("")
    }

    pub fn bounds(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, self.h)
    }

    /// Bounds expanded to include shadow layers (used by dirty-rect engine).
    pub fn bounds_with_shadow(&self) -> Rect {
        let t = &theme::DARK;
        let extra = t.shadow_layers * 2 + 2;
        Rect::new(self.x, self.y, self.w + extra, self.h + extra)
    }

    pub fn titlebar_rect(&self) -> Rect {
        Rect::new(self.x, self.y, self.w, theme::DARK.titlebar_h)
    }

    /// Tick animations. Returns true if any animation changed (needs redraw).
    pub fn tick_animations(&mut self) -> bool {
        // Sync animated position with actual x,y
        self.anim_pos.set_target(self.x as f32, self.y as f32);
        let pos_changed = self.anim_pos.tick();
        let opacity_changed = self.anim_opacity.tick();
        pos_changed || opacity_changed
    }

    /// Animated x position (use for drawing).
    pub fn draw_x(&self) -> usize { self.anim_pos.x.as_usize() }
    /// Animated y position (use for drawing).
    pub fn draw_y(&self) -> usize { self.anim_pos.y.as_usize() }

    /// Bounds using animated position (for dirty-rect calculation during animation).
    pub fn animated_bounds_with_shadow(&self) -> Rect {
        let t = &theme::DARK;
        let extra = t.shadow_layers * 2 + 2;
        Rect::new(self.draw_x(), self.draw_y(), self.w + extra, self.h + extra)
    }

    /// Start the open animation (fade in).
    pub fn animate_open(&mut self) {
        self.anim_opacity.fade_in();
    }

    /// Render the window to its offscreen buffer (only if dirty).
    pub fn render_to_buffer(&mut self, fb: &mut FramebufferManager) {
        if !self.visible || !self.dirty { return; }

        // Allocate buffer on first use
        let buf_pixels = self.buf_w * self.buf_h * 3;
        if self.buffer.len() != buf_pixels {
            self.buffer.resize(buf_pixels, 0);
        }

        // Draw to the global FB at position 0,0 temporarily — we'll copy back.
        // This reuses existing renderer primitives without rewriting them.
        // We render at the animated position directly.
        self.draw_to_fb(fb);

        self.dirty = false;
    }

    /// Draw the window directly to the framebuffer (at animated position).
    pub fn draw(&self, fb: &mut FramebufferManager) {
        self.draw_to_fb(fb);
    }

    /// Internal: draw all window elements to the framebuffer.
    fn draw_to_fb(&self, fb: &mut FramebufferManager) {
        if !self.visible { return; }

        let t = &theme::DARK;
        let r = t.corner_radius;

        // Use animated position for drawing
        let dx = self.draw_x();
        let dy = self.draw_y();

        // Shadow (offset dark layers behind the window)
        for i in 1..=t.shadow_layers {
            let sc = Color::rgba(0, 0, 0, 40u8.saturating_sub(i as u8 * 10));
            renderer::draw_rounded_rect(fb,
                dx + i * 2, dy + i * 2,
                self.w, self.h, r + 2, sc);
        }

        // Window body
        renderer::draw_rounded_rect(fb, dx, dy, self.w, self.h, r, t.window_bg);

        // Titlebar
        let tb_color = if self.active { t.titlebar_active } else { t.titlebar };
        renderer::draw_rounded_rect(fb, dx, dy, self.w, t.titlebar_h, r, tb_color);
        // Fill bottom corners of titlebar (they overlap the body)
        renderer::draw_rect(fb, dx, dy + t.titlebar_h - r, self.w, r, tb_color);

        // Subtle border
        renderer::draw_rounded_border(fb, dx, dy, self.w, self.h, r, t.border);

        // Titlebar separator line
        renderer::draw_hline(fb, dx + 1, dy + t.titlebar_h - 1, self.w - 2,
            Color::rgb(50, 50, 58));

        // Traffic-light buttons (macOS-style) with icons
        let btn_y = dy + (t.titlebar_h - 12) / 2;
        let btn_r = 6;
        
        // Close button (red circle + X icon)
        renderer::draw_circle(fb, dx + 18, dy + t.titlebar_h / 2, btn_r, t.close_btn);
        crate::gui::icons::draw_icon(fb, &crate::gui::icons::ICON_CLOSE, 
            dx + 14, btn_y, renderer::Color::rgb(100, 20, 20));
        
        // Minimize button (yellow circle + - icon)
        renderer::draw_circle(fb, dx + 38, dy + t.titlebar_h / 2, btn_r, t.minimize_btn);
        crate::gui::icons::draw_icon(fb, &crate::gui::icons::ICON_MINIMIZE,
            dx + 34, btn_y, renderer::Color::rgb(120, 90, 20));
        
        // Maximize button (green circle + square icon)
        renderer::draw_circle(fb, dx + 58, dy + t.titlebar_h / 2, btn_r, t.maximize_btn);
        crate::gui::icons::draw_icon(fb, &crate::gui::icons::ICON_MAXIMIZE,
            dx + 54, btn_y, renderer::Color::rgb(20, 80, 20));

        // Title text (centered in titlebar)
        let title = self.title_str();
        let text_w = title.len() * 8; // scale=1, 8px per char
        let text_x = dx + (self.w.saturating_sub(text_w)) / 2;
        let text_y = dy + (t.titlebar_h.saturating_sub(8)) / 2;
        renderer::draw_text(fb, title, text_x, text_y, t.text, 1);

        // Content text
        let cx = dx + t.padding;
        let cy = dy + t.titlebar_h + t.padding;
        let content = self.content_str();
        // Simple multi-line: split by \n or wrap by width
        let max_chars_per_line = (self.w - 2 * t.padding) / 8;
        let mut line_y = cy;
        let mut col = 0usize;
        for &byte in content.as_bytes() {
            if byte == b'\n' || col >= max_chars_per_line {
                line_y += 12; // line height
                col = 0;
                if byte == b'\n' { continue; }
            }
            renderer::draw_char(fb, byte, cx + col * 8, line_y, t.text_dim, 1);
            col += 1;
        }
    }

    /// Handle an event. Returns true if the event was consumed.
    pub fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> bool {
        match *event {
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                if self.titlebar_rect().contains(x, y) {
                    self.dragging = true;
                    self.drag_ox = x.saturating_sub(self.x);
                    self.drag_oy = y.saturating_sub(self.y);
                    return true;
                }
                if self.bounds().contains(x, y) {
                    return true; // consumed but no drag
                }
                false
            }
            Event::MouseUp { button: MouseButton::Left, .. } => {
                if self.dragging {
                    self.dragging = false;
                    return true;
                }
                false
            }
            Event::MouseMove { x, y } => {
                if self.dragging {
                    let new_x = x.saturating_sub(self.drag_ox);
                    let new_y = y.saturating_sub(self.drag_oy);
                    // Clamp to screen
                    self.x = new_x.min(screen_w.saturating_sub(80));
                    self.y = new_y.min(screen_h.saturating_sub(40));
                    return true;
                }
                false
            }
            Event::KeyPress { key } => {
                // Handle keyboard input when window is active
                use crate::gui::event::Key;
                match key {
                    Key::Char(c) => {
                        // Append character to content if there's space
                        if self.content_len < MAX_CONTENT {
                            self.content[self.content_len] = c as u8;
                            self.content_len += 1;
                            self.dirty = true;
                            return true;
                        }
                    }
                    Key::Backspace => {
                        // Remove last character
                        if self.content_len > 0 {
                            self.content_len -= 1;
                            self.dirty = true;
                            return true;
                        }
                    }
                    Key::Enter => {
                        // Add newline
                        if self.content_len < MAX_CONTENT {
                            self.content[self.content_len] = b'\n';
                            self.content_len += 1;
                            self.dirty = true;
                            return true;
                        }
                    }
                    _ => {
                        // Other keys not handled yet
                        return false;
                    }
                }
                false
            }
            _ => false,
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Mark this window as needing a redraw (e.g. after active state change).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}
