// =============================================================================
// Florynx Kernel — GUI Window Component
// =============================================================================
// Draggable, resizable window with rounded titlebar, traffic-light buttons,
// and content area. Supports edge/corner resize, snap-to-edge, and smooth
// LERP-based animations.
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

/// Resize grab zone thickness (pixels from edge).
const RESIZE_GRAB: usize = 6;
/// Minimum window dimensions.
const MIN_W: usize = 160;
const MIN_H: usize = 100;

// ---------------------------------------------------------------------------
// Resize Edge Detection
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeEdge {
    None,
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    pub fn is_resizing(&self) -> bool {
        *self != ResizeEdge::None
    }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

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
    // resize state
    resizing: ResizeEdge,
    resize_origin_x: usize,
    resize_origin_y: usize,
    resize_origin_w: usize,
    resize_origin_h: usize,
    resize_mouse_x: usize,
    resize_mouse_y: usize,
    // snap / maximize state
    pub maximized: bool,
    pre_snap_rect: Option<(usize, usize, usize, usize)>, // (x, y, w, h) before snap
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
    /// Optional userland rectangle primitive: (x, y, w, h, rgb).
    pub user_rect: Option<(usize, usize, usize, usize, u32)>,
    /// True if this window is created from userland syscalls.
    pub user_owned: bool,
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
            resizing: ResizeEdge::None,
            resize_origin_x: 0,
            resize_origin_y: 0,
            resize_origin_w: 0,
            resize_origin_h: 0,
            resize_mouse_x: 0,
            resize_mouse_y: 0,
            maximized: false,
            pre_snap_rect: None,
            anim_pos: AnimatedPos::new(x as f32, y as f32, DRAG_SPEED),
            anim_opacity: AnimatedOpacity::new(0.0, OPEN_SPEED),
            buffer: Vec::new(), // allocated lazily on first render
            buf_w,
            buf_h,
            dirty: true, // needs initial draw
            user_rect: None,
            user_owned: false,
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

    // -----------------------------------------------------------------------
    // Edge / Corner hit testing for resize
    // -----------------------------------------------------------------------

    /// Test if a point is on a resize edge/corner (6px grab zone).
    pub fn edge_hit_test(&self, px: usize, py: usize) -> ResizeEdge {
        if self.maximized { return ResizeEdge::None; }

        let b = self.bounds();
        let g = RESIZE_GRAB;

        // Must be within extended bounds (grab zone extends outside the window)
        let in_x = px + g >= b.x && px < b.x + b.w + g;
        let in_y = py + g >= b.y && py < b.y + b.h + g;
        if !in_x || !in_y { return ResizeEdge::None; }

        let on_left   = px >= b.x.saturating_sub(g) && px < b.x + g;
        let on_right  = px >= b.x + b.w - g && px < b.x + b.w + g;
        let on_top    = py >= b.y.saturating_sub(g) && py < b.y + g;
        let on_bottom = py >= b.y + b.h - g && py < b.y + b.h + g;

        match (on_left, on_right, on_top, on_bottom) {
            (true,  false, true,  false) => ResizeEdge::TopLeft,
            (false, true,  true,  false) => ResizeEdge::TopRight,
            (true,  false, false, true)  => ResizeEdge::BottomLeft,
            (false, true,  false, true)  => ResizeEdge::BottomRight,
            (true,  false, false, false) => ResizeEdge::Left,
            (false, true,  false, false) => ResizeEdge::Right,
            (false, false, true,  false) => ResizeEdge::Top,
            (false, false, false, true)  => ResizeEdge::Bottom,
            _ => ResizeEdge::None,
        }
    }

    // -----------------------------------------------------------------------
    // Traffic-light button hit tests
    // -----------------------------------------------------------------------

    fn button_center_y(&self) -> usize {
        self.y + theme::DARK.titlebar_h / 2
    }

    pub fn close_button_hit(&self, px: usize, py: usize) -> bool {
        let cx = self.x + 18;
        let cy = self.button_center_y();
        let dx = (px as i32 - cx as i32).abs();
        let dy = (py as i32 - cy as i32).abs();
        dx <= 7 && dy <= 7
    }

    pub fn minimize_button_hit(&self, px: usize, py: usize) -> bool {
        let cx = self.x + 38;
        let cy = self.button_center_y();
        let dx = (px as i32 - cx as i32).abs();
        let dy = (py as i32 - cy as i32).abs();
        dx <= 7 && dy <= 7
    }

    pub fn maximize_button_hit(&self, px: usize, py: usize) -> bool {
        let cx = self.x + 58;
        let cy = self.button_center_y();
        let dx = (px as i32 - cx as i32).abs();
        let dy = (py as i32 - cy as i32).abs();
        dx <= 7 && dy <= 7
    }

    // -----------------------------------------------------------------------
    // Snap / Maximize helpers
    // -----------------------------------------------------------------------

    /// Set window bounds in one call (used for snap/maximize).
    pub fn set_bounds(&mut self, x: usize, y: usize, w: usize, h: usize) {
        self.x = x;
        self.y = y;
        self.w = w.max(MIN_W);
        self.h = h.max(MIN_H);
        self.update_buffer_size();
        self.anim_pos.snap(x as f32, y as f32);
        self.dirty = true;
    }

    /// Save current bounds for restore-on-unsnap.
    pub fn save_pre_snap(&mut self) {
        if self.pre_snap_rect.is_none() {
            self.pre_snap_rect = Some((self.x, self.y, self.w, self.h));
        }
    }

    /// Restore pre-snap bounds.
    pub fn restore_pre_snap(&mut self) {
        if let Some((x, y, w, h)) = self.pre_snap_rect.take() {
            self.set_bounds(x, y, w, h);
            self.maximized = false;
        }
    }

    /// Toggle maximized state.
    pub fn toggle_maximize(&mut self, screen_w: usize, screen_h: usize) {
        let menu_h = theme::DARK.menubar_h;
        let dock_h = theme::DARK.dock_h + theme::DARK.dock_margin + 10;
        if self.maximized {
            self.restore_pre_snap();
        } else {
            self.save_pre_snap();
            self.set_bounds(0, menu_h, screen_w, screen_h.saturating_sub(menu_h + dock_h));
            self.maximized = true;
        }
    }

    /// Snap to left/right half of screen.
    pub fn snap_half(&mut self, left: bool, screen_w: usize, screen_h: usize) {
        let menu_h = theme::DARK.menubar_h;
        let dock_h = theme::DARK.dock_h + theme::DARK.dock_margin + 10;
        let half_w = screen_w / 2;
        let usable_h = screen_h.saturating_sub(menu_h + dock_h);
        self.save_pre_snap();
        if left {
            self.set_bounds(0, menu_h, half_w, usable_h);
        } else {
            self.set_bounds(half_w, menu_h, half_w, usable_h);
        }
        self.maximized = false;
    }

    // -----------------------------------------------------------------------
    // Buffer management
    // -----------------------------------------------------------------------

    fn update_buffer_size(&mut self) {
        let th = &theme::DARK;
        let extra = th.shadow_layers * 2 + 2;
        self.buf_w = self.w + extra;
        self.buf_h = self.h + extra;
        // Force reallocation on next render
        self.buffer.clear();
    }

    // -----------------------------------------------------------------------
    // Animations
    // -----------------------------------------------------------------------

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

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Render the window to its offscreen buffer (only if dirty).
    pub fn render_to_buffer(&mut self, fb: &mut FramebufferManager) {
        if !self.visible || !self.dirty { return; }

        // Allocate buffer on first use
        let buf_pixels = self.buf_w * self.buf_h * 3;
        if self.buffer.len() != buf_pixels {
            self.buffer.resize(buf_pixels, 0);
        }

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

        // Title text (centered in titlebar, using AA font if available)
        let title = self.title_str();
        let text_w = renderer::measure_text_aa(title, renderer::FontSize::Title);
        let text_x = dx + (self.w.saturating_sub(text_w)) / 2;
        let text_y = dy + (t.titlebar_h.saturating_sub(14)) / 2;
        renderer::draw_text_aa(fb, title, text_x, text_y, t.text, renderer::FontSize::Title);

        // Content text (using AA font)
        let cx = dx + t.padding;
        let cy = dy + t.titlebar_h + t.padding;
        let content = self.content_str();
        let max_chars_per_line = (self.w - 2 * t.padding) / 7; // proportional ~7px avg
        let mut line_y = cy;
        let mut col = 0usize;
        let mut line_start = 0usize;
        let bytes = content.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'\n' || col >= max_chars_per_line {
                // Render the accumulated line
                if col > 0 || bytes[i] == b'\n' {
                    let end = if bytes[i] == b'\n' { i } else { i };
                    if line_start < end {
                        let line_text = core::str::from_utf8(&bytes[line_start..end]).unwrap_or("");
                        renderer::draw_text_aa(fb, line_text, cx, line_y, t.text_dim, renderer::FontSize::Normal);
                    }
                }
                line_y += 16; // line height for Normal font
                col = 0;
                if bytes[i] == b'\n' {
                    i += 1;
                    line_start = i;
                    continue;
                }
                line_start = i;
            }
            col += 1;
            i += 1;
        }
        // Render remaining text
        if line_start < bytes.len() {
            let line_text = core::str::from_utf8(&bytes[line_start..]).unwrap_or("");
            renderer::draw_text_aa(fb, line_text, cx, line_y, t.text_dim, renderer::FontSize::Normal);
        }

        // Optional userland rectangle draw primitive (inside content area).
        if let Some((rx, ry, rw, rh, rgb)) = self.user_rect {
            let r = ((rgb >> 16) & 0xFF) as u8;
            let g = ((rgb >> 8) & 0xFF) as u8;
            let b = (rgb & 0xFF) as u8;
            let draw_x = cx + rx;
            let draw_y = cy + ry;
            renderer::draw_rect(fb, draw_x, draw_y, rw, rh, Color::rgb(r, g, b));
        }
    }

    // -----------------------------------------------------------------------
    // Event handling
    // -----------------------------------------------------------------------

    /// Handle an event. Returns true if the event was consumed.
    pub fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> bool {
        match *event {
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                // Check resize edges first (higher priority than titlebar drag)
                let edge = self.edge_hit_test(x, y);
                if edge.is_resizing() {
                    self.resizing = edge;
                    self.resize_origin_x = self.x;
                    self.resize_origin_y = self.y;
                    self.resize_origin_w = self.w;
                    self.resize_origin_h = self.h;
                    self.resize_mouse_x = x;
                    self.resize_mouse_y = y;
                    return true;
                }
                // Check titlebar for drag (but not on traffic-light buttons)
                if self.titlebar_rect().contains(x, y) {
                    // Check traffic-light buttons
                    if self.close_button_hit(x, y)
                        || self.minimize_button_hit(x, y)
                        || self.maximize_button_hit(x, y)
                    {
                        // Button clicks handled separately in desktop.rs
                        return true;
                    }
                    // Unsnap on drag if maximized
                    if self.maximized {
                        // Restore but center window under cursor
                        if let Some((_, _, pw, ph)) = self.pre_snap_rect {
                            let new_x = x.saturating_sub(pw / 2);
                            let new_y = y.saturating_sub(theme::DARK.titlebar_h / 2);
                            self.restore_pre_snap();
                            self.x = new_x;
                            self.y = new_y;
                            self.w = pw;
                            self.h = ph;
                            self.anim_pos.snap(new_x as f32, new_y as f32);
                        }
                    }
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
                if self.resizing.is_resizing() {
                    self.resizing = ResizeEdge::None;
                    self.update_buffer_size();
                    self.dirty = true;
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
                if self.resizing.is_resizing() {
                    self.do_resize(x, y, screen_w, screen_h);
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

    /// Execute resize based on mouse delta.
    fn do_resize(&mut self, mx: usize, my: usize, screen_w: usize, screen_h: usize) {
        let dx = mx as i64 - self.resize_mouse_x as i64;
        let dy = my as i64 - self.resize_mouse_y as i64;

        let orig_x = self.resize_origin_x as i64;
        let orig_y = self.resize_origin_y as i64;
        let orig_w = self.resize_origin_w as i64;
        let orig_h = self.resize_origin_h as i64;

        let (mut new_x, mut new_y, mut new_w, mut new_h) = (orig_x, orig_y, orig_w, orig_h);

        match self.resizing {
            ResizeEdge::Right => {
                new_w = (orig_w + dx).max(MIN_W as i64);
            }
            ResizeEdge::Bottom => {
                new_h = (orig_h + dy).max(MIN_H as i64);
            }
            ResizeEdge::Left => {
                new_w = (orig_w - dx).max(MIN_W as i64);
                new_x = orig_x + orig_w - new_w;
            }
            ResizeEdge::Top => {
                new_h = (orig_h - dy).max(MIN_H as i64);
                new_y = orig_y + orig_h - new_h;
            }
            ResizeEdge::TopLeft => {
                new_w = (orig_w - dx).max(MIN_W as i64);
                new_h = (orig_h - dy).max(MIN_H as i64);
                new_x = orig_x + orig_w - new_w;
                new_y = orig_y + orig_h - new_h;
            }
            ResizeEdge::TopRight => {
                new_w = (orig_w + dx).max(MIN_W as i64);
                new_h = (orig_h - dy).max(MIN_H as i64);
                new_y = orig_y + orig_h - new_h;
            }
            ResizeEdge::BottomLeft => {
                new_w = (orig_w - dx).max(MIN_W as i64);
                new_h = (orig_h + dy).max(MIN_H as i64);
                new_x = orig_x + orig_w - new_w;
            }
            ResizeEdge::BottomRight => {
                new_w = (orig_w + dx).max(MIN_W as i64);
                new_h = (orig_h + dy).max(MIN_H as i64);
            }
            ResizeEdge::None => {}
        }

        // Clamp to screen
        new_x = new_x.max(0).min(screen_w as i64 - MIN_W as i64);
        new_y = new_y.max(0).min(screen_h as i64 - MIN_H as i64);
        new_w = new_w.min(screen_w as i64 - new_x);
        new_h = new_h.min(screen_h as i64 - new_y);

        self.x = new_x as usize;
        self.y = new_y as usize;
        self.w = new_w as usize;
        self.h = new_h as usize;
        self.dirty = true;
    }

    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    pub fn is_resizing(&self) -> bool {
        self.resizing.is_resizing()
    }

    /// Mark this window as needing a redraw (e.g. after active state change).
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn set_user_rect(&mut self, x: usize, y: usize, w: usize, h: usize, rgb: u32) {
        self.user_rect = Some((x, y, w, h, rgb));
        self.dirty = true;
    }

    pub fn set_user_owned(&mut self, user_owned: bool) {
        self.user_owned = user_owned;
    }
}
