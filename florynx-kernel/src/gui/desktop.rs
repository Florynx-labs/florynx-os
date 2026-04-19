// =============================================================================
// Florynx Kernel — Desktop Compositor + Window Manager
// =============================================================================
// Manages the desktop background, window z-ordering, dock, and cursor.
// Provides the main draw() and handle_event() entry points for the GUI.
// =============================================================================

use alloc::vec::Vec;
use spin::Mutex;
use crate::gui::renderer::{self, Color, FramebufferManager, FRAMEBUFFER};
use crate::gui::theme;
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::window::Window;
use crate::gui::dock::Dock;
use crate::gui::menubar::MenuBar;

const MAX_WINDOWS: usize = 16;

const WALLPAPER_W: usize = 1024;
const WALLPAPER_H: usize = 768;
const WALLPAPER_RGB: &[u8] = include_bytes!("assets/wallpaper_1024x768.rgb");

// ---------------------------------------------------------------------------
// Window Manager
// ---------------------------------------------------------------------------

pub struct WindowManager {
    windows: Vec<Option<Window>>,
    /// Z-order: indices into `windows`, front-most first.
    order: [usize; MAX_WINDOWS],
    count: usize,
    active: Option<usize>,
    next_id: usize,
}

impl WindowManager {
    fn new() -> Self {
        let mut windows = Vec::with_capacity(MAX_WINDOWS);
        for _ in 0..MAX_WINDOWS { windows.push(None); }
        
        WindowManager {
            windows,
            order: [0; MAX_WINDOWS],
            count: 0,
            active: None,
            next_id: 1,
        }
    }

    /// Add a window and bring it to front. Returns the window id.
    pub fn add_window(&mut self, mut win: Window) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        win.id = id;
        win.active = true;

        // Deactivate current active
        if let Some(ai) = self.active {
            if let Some(ref mut w) = self.windows[ai] {
                w.active = false;
                w.mark_dirty();
            }
        }

        // Find free slot
        for i in 0..MAX_WINDOWS {
            if self.windows[i].is_none() {
                self.windows[i] = Some(win);
                // Insert at front of z-order
                // Shift existing order right
                let c = self.count;
                let mut j = c;
                while j > 0 {
                    self.order[j] = self.order[j - 1];
                    j -= 1;
                }
                self.order[0] = i;
                self.count += 1;
                self.active = Some(i);
                return id;
            }
        }
        0 // no free slot
    }

    /// Bring a window to the front of the z-order.
    fn bring_to_front(&mut self, slot: usize) -> bool {
        // Find position in order
        let mut pos = None;
        for i in 0..self.count {
            if self.order[i] == slot {
                pos = Some(i);
                break;
            }
        }
        
        let changed = if let Some(p) = pos {
            if p == 0 { return false; } // Already at front
            // Shift left from front to p, then place at front
            let val = self.order[p];
            let mut i = p;
            while i > 0 {
                self.order[i] = self.order[i - 1];
                i -= 1;
            }
            self.order[0] = val;
            true
        } else {
            false
        };

        // Update active
        if let Some(ai) = self.active {
            if let Some(ref mut w) = self.windows[ai] {
                w.active = false;
                w.mark_dirty();
            }
        }
        if let Some(ref mut w) = self.windows[slot] {
            w.active = true;
            w.mark_dirty();
        }
        self.active = Some(slot);
        changed
    }

    fn find_slot_by_id(&self, win_id: usize) -> Option<usize> {
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.id == win_id {
                    return Some(slot);
                }
            }
        }
        None
    }

    /// Draw all windows back-to-front.
    fn draw(&self, fb: &mut FramebufferManager) {
        // Draw back to front (last in order array = back-most)
        for i in (0..self.count).rev() {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                w.draw(fb);
            }
        }
    }

    /// Dispatch event to windows front-to-back. Returns true if consumed.
    fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> bool {
        // Front-to-back hit testing — find which slot consumed the event
        let mut consumed_slot: Option<usize> = None;
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref mut w) = self.windows[slot] {
                if w.handle_event(event, screen_w, screen_h) {
                    consumed_slot = Some(slot);
                    break;
                }
            }
        }

        if let Some(slot) = consumed_slot {
            // If this was a mouse-down, bring window to front
            if matches!(event, Event::MouseDown { .. }) && self.active != Some(slot) {
                if self.bring_to_front(slot) {
                    // Window brought to front, will be redrawn
                    return true;
                }
            }
            return true;
        }
        false
    }

    /// Check if any window is being dragged.
    fn any_dragging(&self) -> bool {
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.is_dragging() { return true; }
            }
        }
        false
    }

    /// Check if any window is being resized.
    fn any_resizing(&self) -> bool {
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.is_resizing() { return true; }
            }
        }
        false
    }

    /// Check if any window is being dragged OR resized.
    fn any_interactive(&self) -> bool {
        self.any_dragging() || self.any_resizing()
    }

    /// Get the bounds (including shadow) of the currently dragged window, if any.
    fn dragged_bounds_with_shadow(&self) -> Option<Rect> {
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.is_dragging() {
                    return Some(w.bounds_with_shadow());
                }
            }
        }
        None
    }

    /// Get the bounds of the currently resized window, if any.
    fn resized_bounds_with_shadow(&self) -> Option<Rect> {
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.is_resizing() {
                    return Some(w.bounds_with_shadow());
                }
            }
        }
        None
    }

    /// Remove a window by its id. Returns removed bounds for invalidation.
    fn remove_window_by_id(&mut self, win_id: usize) -> Option<Rect> {
        let mut slot_to_remove = None;
        for i in 0..self.count {
            let slot = self.order[i];
            if let Some(ref w) = self.windows[slot] {
                if w.id == win_id {
                    slot_to_remove = Some(slot);
                    break;
                }
            }
        }
        let slot = slot_to_remove?;
        let removed_bounds = self.windows[slot].as_ref().map(|w| w.bounds_with_shadow())?;

        self.windows[slot] = None;

        // Remove slot from z-order list.
        let mut pos = None;
        for i in 0..self.count {
            if self.order[i] == slot {
                pos = Some(i);
                break;
            }
        }
        if let Some(p) = pos {
            for i in p..self.count.saturating_sub(1) {
                self.order[i] = self.order[i + 1];
            }
            self.count = self.count.saturating_sub(1);
        }

        // Recompute active window as front-most, if any.
        self.active = if self.count > 0 { Some(self.order[0]) } else { None };
        if let Some(active_slot) = self.active {
            if let Some(ref mut w) = self.windows[active_slot] {
                w.active = true;
                w.mark_dirty();
            }
        }

        Some(removed_bounds)
    }
}

// ---------------------------------------------------------------------------
// Desktop
// ---------------------------------------------------------------------------

const MAX_DIRTY: usize = 32;

/// Snap zones for window edge-snapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapZone {
    None,
    Left,
    Right,
    Maximize,
}

pub struct Desktop {
    wm: WindowManager,
    dock: Dock,
    menubar: MenuBar,
    screen_w: usize,
    screen_h: usize,
    mouse_x: usize,
    mouse_y: usize,
    prev_buttons: u8,
    needs_full_redraw: bool,
    /// Cached desktop background (RGB bytes, screen_w * screen_h * 3).
    bg_cache: Vec<u8>,
    bg_cached: bool,
    /// Dirty rectangles for partial redraw (avoids full-screen blit on drag).
    dirty: [Rect; MAX_DIRTY],
    dirty_count: usize,
    /// Current snap preview zone (shown as semi-transparent overlay).
    snap_preview: SnapZone,
}

impl Desktop {
    fn new(screen_w: usize, screen_h: usize) -> Self {
        let mut dock = Dock::new();
        // Default dock items with high-fidelity PNG icons
        dock.add_named(include_bytes!("assets/folder.png"), "Files");
        dock.add_named(include_bytes!("assets/terminal.png"), "Terminal");
        dock.add_named(include_bytes!("assets/settings.png"), "Settings");
        dock.add_named(include_bytes!("assets/folder.png"), "Monitor");
        dock.add_named(include_bytes!("assets/folder.png"), "Notes");

        // Mark first item as "active"
        dock.set_active(0, true);

        let mut menubar = MenuBar::new(screen_w);
        menubar.set_title("FlorynxOS");

        // Pre-allocate background cache to the exact size to avoid mid-render reallocs
        let mut bg_cache = Vec::with_capacity(screen_w * screen_h * 3);
        unsafe { bg_cache.set_len(screen_w * screen_h * 3); }

        Desktop {
            wm: WindowManager::new(),
            dock,
            menubar,
            screen_w,
            screen_h,
            mouse_x: screen_w / 2,
            mouse_y: screen_h / 2,
            prev_buttons: 0,
            needs_full_redraw: true,
            bg_cache,
            bg_cached: false,
            dirty: [Rect::new(0, 0, 0, 0); MAX_DIRTY],
            dirty_count: 0,
            snap_preview: SnapZone::None,
        }
    }

    /// Add a window to the desktop.
    pub fn add_window(&mut self, mut win: Window) -> usize {
        let wb = win.bounds_with_shadow();
        // Start the open animation (fade in)
        win.animate_open();
        let id = self.wm.add_window(win);
        // Only dirty the new window area + dock (for active dot update)
        self.mark_dirty(wb);
        let dock_y = self.screen_h.saturating_sub(theme::DARK.dock_h + theme::DARK.dock_margin + 10);
        self.mark_dirty(Rect::new(0, dock_y, self.screen_w, self.screen_h - dock_y));
        id
    }

    /// Mark a rectangle as dirty (needs repaint).
    fn mark_dirty(&mut self, r: Rect) {
        let r = r.clamp(self.screen_w, self.screen_h);
        if r.w == 0 || r.h == 0 { return; }
        if self.dirty_count < MAX_DIRTY {
            self.dirty[self.dirty_count] = r;
            self.dirty_count += 1;
        } else {
            // Too many dirty rects — fall back to full redraw
            self.needs_full_redraw = true;
        }
    }

    /// Blit a rectangle from the bg_cache onto the framebuffer.
    fn blit_bg_rect(&self, fb: &mut FramebufferManager, r: &Rect) {
        let sw = self.screen_w;
        for y in r.y..(r.y + r.h).min(self.screen_h) {
            for x in r.x..(r.x + r.w).min(sw) {
                let idx = (y * sw + x) * 3;
                fb.set_pixel(x, y,
                    self.bg_cache[idx],
                    self.bg_cache[idx + 1],
                    self.bg_cache[idx + 2]);
            }
        }
    }

    /// Full draw: background + all windows + dock → flush entire back buffer to VRAM.
    pub fn draw_full(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;

        if !self.bg_cached {
            // Blit the embedded wallpaper image to the back buffer.
            // Falls back to gradient if the wallpaper doesn't match screen size.
            let expected = WALLPAPER_W * WALLPAPER_H * 3;
            if WALLPAPER_RGB.len() >= expected
                && self.screen_w == WALLPAPER_W
                && self.screen_h == WALLPAPER_H
            {
                for y in 0..WALLPAPER_H {
                    for x in 0..WALLPAPER_W {
                        let i = (y * WALLPAPER_W + x) * 3;
                        fb.set_pixel(x, y, WALLPAPER_RGB[i], WALLPAPER_RGB[i + 1], WALLPAPER_RGB[i + 2]);
                    }
                }
            } else {
                renderer::draw_gradient_with_noise(fb, 0, 0, self.screen_w, self.screen_h,
                    t.desktop_top, t.desktop_bot, 8);
                renderer::draw_vignette(fb, 40);
            }

            // Cache the rendered background
            let total = self.screen_w * self.screen_h * 3;
            self.bg_cache.resize(total, 0);
            for y in 0..self.screen_h {
                for x in 0..self.screen_w {
                    let (r, g, b) = fb.get_pixel(x, y);
                    let idx = (y * self.screen_w + x) * 3;
                    self.bg_cache[idx] = r;
                    self.bg_cache[idx + 1] = g;
                    self.bg_cache[idx + 2] = b;
                }
            }
            self.bg_cached = true;
        } else {
            // Blit full cached background to back buffer
            let full = Rect::new(0, 0, self.screen_w, self.screen_h);
            self.blit_bg_rect(fb, &full);
        }

        self.wm.draw(fb);
        self.dock.draw(fb, self.screen_w, self.screen_h);
        self.menubar.draw(fb);

        // Draw snap preview overlay if active
        self.draw_snap_preview(fb);

        // Flush entire back buffer to VRAM in one shot
        fb.flush_full();

        self.needs_full_redraw = false;
        self.dirty_count = 0;
    }

    /// Partial draw: only repaint dirty rectangles, then flush them to VRAM.
    fn draw_partial(&mut self, fb: &mut FramebufferManager) {
        if self.dirty_count == 0 { return; }

        // Merge overlapping dirty rects to reduce flush calls
        self.merge_dirty_rects();

        // 1. Blit background cache over each dirty rect (erase old content in back buffer)
        for i in 0..self.dirty_count {
            self.blit_bg_rect(fb, &self.dirty[i]);
        }

        // 2. Redraw windows that intersect any dirty rect (back to front)
        for wi in (0..self.wm.count).rev() {
            let slot = self.wm.order[wi];
            if let Some(ref w) = self.wm.windows[slot] {
                let wb = w.bounds_with_shadow();
                let mut overlaps = false;
                for i in 0..self.dirty_count {
                    if wb.intersects(&self.dirty[i]) {
                        overlaps = true;
                        break;
                    }
                }
                if overlaps {
                    w.draw(fb);
                }
            }
        }

        // 3. Redraw dock if any dirty rect overlaps it
        let dock_y = self.screen_h.saturating_sub(theme::DARK.dock_h + theme::DARK.dock_margin + 10);
        let dock_rect = Rect::new(0, dock_y, self.screen_w, self.screen_h - dock_y);
        for i in 0..self.dirty_count {
            if dock_rect.intersects(&self.dirty[i]) {
                self.dock.draw(fb, self.screen_w, self.screen_h);
                break;
            }
        }

        // 3b. Redraw menu bar if any dirty rect overlaps it (always on top)
        let mb_rect = self.menubar.rect();
        for i in 0..self.dirty_count {
            if mb_rect.intersects(&self.dirty[i]) {
                self.menubar.draw(fb);
                break;
            }
        }

        // 4. Flush ONLY dirty regions from back buffer → VRAM
        for i in 0..self.dirty_count {
            let r = self.dirty[i];
            fb.flush_rect(r.x, r.y, r.w, r.h);
        }

        self.dirty_count = 0;
    }

    /// Merge overlapping dirty rects to minimize flush calls.
    fn merge_dirty_rects(&mut self) {
        if self.dirty_count <= 1 { return; }

        let mut merged = true;
        while merged {
            merged = false;
            let mut i = 0;
            while i < self.dirty_count {
                let mut j = i + 1;
                while j < self.dirty_count {
                    if self.dirty[i].intersects(&self.dirty[j]) {
                        // Merge j into i
                        self.dirty[i] = self.dirty[i].union(&self.dirty[j]);
                        // Remove j by swapping with last
                        self.dirty_count -= 1;
                        self.dirty[j] = self.dirty[self.dirty_count];
                        merged = true;
                    } else {
                        j += 1;
                    }
                }
                i += 1;
            }
        }
    }

    /// Process keyboard input from the PS/2 keyboard driver.
    pub fn on_key(&mut self, key: crate::gui::event::Key) {
        use crate::gui::event::Event;
        
        // Dispatch key event to the active window
        if let Some(active_idx) = self.wm.active {
            if let Some(ref mut win) = self.wm.windows[active_idx] {
                let event = Event::KeyPress { key };
                if win.handle_event(&event, self.screen_w, self.screen_h) {
                    // Only dirty the content area (below titlebar), not the whole window+shadow
                    let t = &theme::DARK;
                    let content_rect = Rect::new(
                        win.x,
                        win.y + t.titlebar_h,
                        win.w,
                        win.h.saturating_sub(t.titlebar_h),
                    );
                    self.mark_dirty(content_rect);
                }
            }
        }
    }

    /// Process raw mouse state from the PS/2 driver.
    pub fn on_mouse(&mut self, x: usize, y: usize, buttons: u8) {
        let old_buttons = self.prev_buttons;
        self.prev_buttons = buttons;

        let left_now = buttons & 1 != 0;
        let left_was = old_buttons & 1 != 0;

        if left_now && !left_was {
            let ev = Event::MouseDown { x, y, button: MouseButton::Left };

            // Check dock first
            if let Some(icon_idx) = self.dock.handle_event(&ev, self.screen_w, self.screen_h) {
                self.on_dock_click(icon_idx);
                return;
            }

            // Check traffic-light buttons BEFORE general window event dispatch
            if let Some(active_slot) = self.wm.active {
                let mut action: Option<u8> = None; // 0=close, 1=minimize, 2=maximize
                if let Some(ref w) = self.wm.windows[active_slot] {
                    if w.close_button_hit(x, y) {
                        action = Some(0);
                    } else if w.minimize_button_hit(x, y) {
                        action = Some(1);
                    } else if w.maximize_button_hit(x, y) {
                        action = Some(2);
                    }
                }
                match action {
                    Some(0) => {
                        // Close window
                        let win_id = self.wm.windows[active_slot].as_ref().map(|w| w.id).unwrap_or(0);
                        if let Some(bounds) = self.wm.remove_window_by_id(win_id) {
                            self.mark_dirty(bounds);
                            self.needs_full_redraw = true;
                            crate::gui::event_bus::push_user_window_destroyed(win_id as u32);
                        }
                        self.sync_menubar_title();
                        return;
                    }
                    Some(1) => {
                        // Minimize: toggle visibility
                        if let Some(ref mut w) = self.wm.windows[active_slot] {
                            let bounds = w.bounds_with_shadow();
                            w.visible = !w.visible;
                            w.mark_dirty();
                            self.mark_dirty(bounds);
                            self.needs_full_redraw = true;
                        }
                        return;
                    }
                    Some(2) => {
                        // Maximize/restore
                        let sw = self.screen_w;
                        let sh = self.screen_h;
                        if let Some(ref mut w) = self.wm.windows[active_slot] {
                            let old_bounds = w.bounds_with_shadow();
                            w.toggle_maximize(sw, sh);
                            let new_bounds = w.bounds_with_shadow();
                            self.mark_dirty(old_bounds);
                            self.mark_dirty(new_bounds);
                            self.needs_full_redraw = true;
                        }
                        return;
                    }
                    _ => {}
                }
            }

            // Check windows — if a click brings a window to front, dirty all overlapping areas
            let old_active = self.wm.active;
            if self.wm.handle_event(&ev, self.screen_w, self.screen_h) {
                if self.wm.active != old_active {
                    // Focus changed — dirty both old and new active windows
                    if let Some(old_idx) = old_active {
                        if let Some(ref w) = self.wm.windows[old_idx] {
                            self.mark_dirty(w.bounds_with_shadow());
                        }
                    }
                    if let Some(new_idx) = self.wm.active {
                        if let Some(ref w) = self.wm.windows[new_idx] {
                            self.mark_dirty(w.bounds_with_shadow());
                        }
                        // Emit focus event to userland
                        if let Some(ref w) = self.wm.windows[new_idx] {
                            crate::gui::event_bus::push_user_window_focused(w.id as u32);
                        }
                    }
                    self.sync_menubar_title();
                }
            }
        }

        if !left_now && left_was {
            // On mouse-up: check if we were dragging and should snap
            if self.wm.any_dragging() && self.snap_preview != SnapZone::None {
                // Apply snap to the dragged window
                let sw = self.screen_w;
                let sh = self.screen_h;
                let snap = self.snap_preview;
                for i in 0..self.wm.count {
                    let slot = self.wm.order[i];
                    if let Some(ref mut w) = self.wm.windows[slot] {
                        if w.is_dragging() {
                            let old_b = w.bounds_with_shadow();
                            match snap {
                                SnapZone::Left => w.snap_half(true, sw, sh),
                                SnapZone::Right => w.snap_half(false, sw, sh),
                                SnapZone::Maximize => w.toggle_maximize(sw, sh),
                                SnapZone::None => {}
                            }
                            let new_b = w.bounds_with_shadow();
                            self.mark_dirty(old_b);
                            self.mark_dirty(new_b);
                            self.needs_full_redraw = true;
                            break;
                        }
                    }
                }
                self.snap_preview = SnapZone::None;
            }

            let ev = Event::MouseUp { x, y, button: MouseButton::Left };
            self.wm.handle_event(&ev, self.screen_w, self.screen_h);
        }

        if x != self.mouse_x || y != self.mouse_y {
            // Mark old + new cursor regions dirty so the cursor redraws cleanly
            let cw = 14;
            let ch = 20;
            self.mark_dirty(Rect::new(self.mouse_x, self.mouse_y, cw, ch));
            self.mark_dirty(Rect::new(x, y, cw, ch));

            self.mouse_x = x;
            self.mouse_y = y;
            let ev = Event::MouseMove { x, y };

            if self.wm.any_interactive() {
                // Save old window bounds BEFORE the move
                let old_drag_bounds = self.wm.dragged_bounds_with_shadow();
                let old_resize_bounds = self.wm.resized_bounds_with_shadow();

                // Process the move (updates window position or size)
                self.wm.handle_event(&ev, self.screen_w, self.screen_h);

                // Mark old position + new position as dirty
                if let Some(old_r) = old_drag_bounds {
                    self.mark_dirty(old_r);
                }
                if let Some(new_r) = self.wm.dragged_bounds_with_shadow() {
                    self.mark_dirty(new_r);
                }
                if let Some(old_r) = old_resize_bounds {
                    self.mark_dirty(old_r);
                }
                if let Some(new_r) = self.wm.resized_bounds_with_shadow() {
                    self.mark_dirty(new_r);
                }

                // Snap zone detection during drag
                if self.wm.any_dragging() {
                    let old_snap = self.snap_preview;
                    self.snap_preview = if x <= 2 {
                        SnapZone::Left
                    } else if x >= self.screen_w.saturating_sub(3) {
                        SnapZone::Right
                    } else if y <= 2 {
                        SnapZone::Maximize
                    } else {
                        SnapZone::None
                    };
                    if self.snap_preview != old_snap {
                        // Dirty the snap preview area
                        self.needs_full_redraw = true;
                    }
                }
            } else {
                // Update dock hover state
                let old_hovered = self.dock.hovered;
                self.dock.handle_event(&ev, self.screen_w, self.screen_h);
                if self.dock.hovered != old_hovered {
                    let dock_y = self.screen_h.saturating_sub(theme::DARK.dock_h + theme::DARK.dock_margin + 10);
                    self.mark_dirty(Rect::new(0, dock_y, self.screen_w, self.screen_h - dock_y));
                }
            }
        }
    }

    /// Draw snap preview overlay.
    fn draw_snap_preview(&self, fb: &mut FramebufferManager) {
        if self.snap_preview == SnapZone::None { return; }

        let menu_h = theme::DARK.menubar_h;
        let dock_h = theme::DARK.dock_h + theme::DARK.dock_margin + 10;
        let usable_h = self.screen_h.saturating_sub(menu_h + dock_h);
        let accent = theme::DARK.accent;
        let preview_color = Color::rgba(accent.r, accent.g, accent.b, 40);

        match self.snap_preview {
            SnapZone::Left => {
                renderer::draw_rounded_rect(fb, 4, menu_h + 4,
                    self.screen_w / 2 - 8, usable_h - 8, 12, preview_color);
            }
            SnapZone::Right => {
                renderer::draw_rounded_rect(fb, self.screen_w / 2 + 4, menu_h + 4,
                    self.screen_w / 2 - 8, usable_h - 8, 12, preview_color);
            }
            SnapZone::Maximize => {
                renderer::draw_rounded_rect(fb, 4, menu_h + 4,
                    self.screen_w - 8, usable_h - 8, 12, preview_color);
            }
            SnapZone::None => {}
        }
    }

    /// Tick all animations (called once per frame). Marks dirty rects for any
    /// window whose animated position/opacity changed.
    pub fn tick_animations(&mut self) {
        // --- Window animations ---
        let mut anim_dirty: [(Rect, Rect, bool); MAX_WINDOWS] =
            [(Rect::new(0,0,0,0), Rect::new(0,0,0,0), false); MAX_WINDOWS];

        for slot in 0..self.wm.windows.len() {
            if let Some(ref mut w) = self.wm.windows[slot] {
                let old_bounds = w.animated_bounds_with_shadow();
                if w.tick_animations() {
                    let new_bounds = w.animated_bounds_with_shadow();
                    anim_dirty[slot] = (old_bounds, new_bounds, true);
                }
            }
        }

        for slot in 0..self.wm.windows.len() {
            let (old_b, new_b, changed) = anim_dirty[slot];
            if changed {
                self.mark_dirty(old_b);
                self.mark_dirty(new_b);
            }
        }

        // --- Dock scale animations ---
        if self.dock.tick_animations() {
            let dock_y = self.screen_h.saturating_sub(theme::DARK.dock_h + theme::DARK.dock_margin + 10);
            self.mark_dirty(Rect::new(0, dock_y, self.screen_w, self.screen_h - dock_y));
        }

        // --- Menu bar clock: dirty every ~200 ticks (1 sec at 200 Hz PIT) ---
        static LAST_SEC: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
        let cur_sec = crate::drivers::timer::pit::get_ticks() / 200;
        let prev = LAST_SEC.load(core::sync::atomic::Ordering::Relaxed);
        if cur_sec != prev {
            LAST_SEC.store(cur_sec, core::sync::atomic::Ordering::Relaxed);
            self.mark_dirty(self.menubar.rect());
        }
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_full_redraw || self.dirty_count > 0
    }

    pub fn request_redraw(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Sync the menu bar title from the active window (or default to "FlorynxOS").
    fn sync_menubar_title(&mut self) {
        if let Some(active_slot) = self.wm.active {
            if let Some(ref w) = self.wm.windows[active_slot] {
                self.menubar.set_title(w.title_str());
            }
        } else {
            self.menubar.set_title("FlorynxOS");
        }
        self.mark_dirty(self.menubar.rect());
    }

    fn active_user_window_id(&self) -> Option<usize> {
        let active = self.wm.active?;
        let w = self.wm.windows[active].as_ref()?;
        if w.user_owned { Some(w.id) } else { None }
    }

    fn active_user_window_slot(&self) -> Option<usize> {
        let active = self.wm.active?;
        let w = self.wm.windows[active].as_ref()?;
        if w.user_owned { Some(active) } else { None }
    }

    fn active_user_window_contains(&self, x: usize, y: usize) -> bool {
        let active = match self.wm.active {
            Some(a) => a,
            None => return false,
        };
        let w = match self.wm.windows[active].as_ref() {
            Some(w) => w,
            None => return false,
        };
        w.user_owned && w.bounds().contains(x, y)
    }

    /// Handle dock icon click - create appropriate window
    fn on_dock_click(&mut self, icon_idx: usize) {
        use crate::gui::window::Window;
        
        let (x, y) = (100 + icon_idx * 30, 100 + icon_idx * 30);
        
        let win = match icon_idx {
            0 => {
                // Files icon
                let mut w = Window::new(0, x, y, 650, 450, "Files");
                w.set_content("Florynx File Manager\n\n[ HOME ]\n- Documents\n- Downloads\n- Pictures\n\n[ SYSTEM ]\n- /dev\n- /bin\n- /tmp\n\n\n(VFS bindings in development - Phase 6)");
                w
            }
            1 => {
                // Terminal icon
                let mut w = Window::new(0, x, y, 650, 420, "Terminal");
                w.is_terminal = true;
                w.set_content("florsh v0.1 — Welcome to Florynx OS\n$ ");
                w
            }
            2 => {
                // Settings icon
                let mut w = Window::new(0, x, y, 500, 450, "Settings");
                w.set_content("System Settings\n\n[ Appearance ]\nTheme: Bioluminescent Glass\nColors: Active\n\n[ Network ]\nWi-Fi: Disconnected\n\n[ Security ]\nCapabilities: Enforcing\n\n(Settings Panel UI expected in Phase 5)");
                w
            }
            3 => {
                // Monitor icon
                let mut w = Window::new(0, x, y, 500, 400, "System Monitor");
                w.set_content("System Monitor (top)\n\nCPU: AMD64 x86_64\nMemory: 16 MiB kernel heap pool\nProcesses: 3 running\n\n[ID]   [NAME]         [MEM]\n  1    kernel_idle    2.1M\n  2    compositor     8.4M\n  3    florsh         1.2M\n\n(Real-time stats coming soon)");
                w
            }
            4 => {
                // Notes/Document icon
                let mut w = Window::new(0, x, y, 550, 400, "Notes");
                w.set_content("Florynx Notes\n\nType your thoughts here...\nSupports Backspace and new lines.\nEverything is seamlessly redrawn through the double-buffered dirty engine!");
                w
            }
            _ => {
                // Default window
                let mut w = Window::new(0, x, y, 400, 300, "Application");
                w.set_content("New Application Window");
                w
            }
        };
        
        self.add_window(win);
        self.dock.set_active(icon_idx, true);
        self.sync_menubar_title();
        
        crate::serial_println!("[desktop] Dock icon {} clicked - created window", icon_idx);
    }
}

// ---------------------------------------------------------------------------
// Global Desktop instance
// ---------------------------------------------------------------------------

pub static DESKTOP: Mutex<Option<Desktop>> = Mutex::new(None);

/// Initialize the desktop GUI. Call after BGA framebuffer is ready.
pub fn init() {
    let (sw, sh) = {
        let guard = FRAMEBUFFER.lock();
        match guard.as_ref() {
            Some(fb) => fb.dimensions(),
            None => return,
        }
    };

    if sw == 0 || sh == 0 { return; }

    let mut desktop = Desktop::new(sw, sh);

    // Create a welcome window
    let mut win = Window::new(0, 50, 50, 440, 280, "Welcome to FlorynxOS");
    win.set_content("Florynx OS v0.2.4\n\nBioluminescent desktop shell\nbuilt from scratch in Rust.\n\nFeatures:\n- Keyboard input\n- Text editor\n- Button widgets\n- Drag windows around!\n\nType in the active window!");
    desktop.add_window(win);

    // Create a text editor window
    let mut editor_win = Window::new(0, sw / 2 - 250, sh / 2 - 200, 500, 400, "Text Editor");
    editor_win.set_content("Type here to test keyboard input!\nPress Enter for new lines.\nBackspace to delete.");
    desktop.add_window(editor_win);

    *DESKTOP.lock() = Some(desktop);

    crate::serial_println!("[desktop] GUI initialized ({}x{})", sw, sh);
}

/// Initialize desktop core without kernel demo windows.
/// This mode is used when handing UI ownership to userland/HGUI.
pub fn init_empty() {
    let (sw, sh) = {
        let guard = FRAMEBUFFER.lock();
        match guard.as_ref() {
            Some(fb) => fb.dimensions(),
            None => return,
        }
    };

    if sw == 0 || sh == 0 {
        return;
    }

    let desktop = Desktop::new(sw, sh);
    *DESKTOP.lock() = Some(desktop);
    crate::serial_println!("[desktop] HGUI core initialized ({}x{})", sw, sh);
}

/// Draw the desktop (called once at startup).
pub fn draw() {
    if let Some(ref mut fb) = *FRAMEBUFFER.lock() {
        if let Some(ref mut desktop) = *DESKTOP.lock() {
            desktop.draw_full(fb); // draws to back buffer + flush_full
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
            fb.flush_rect(desktop.mouse_x, desktop.mouse_y, 14, 20);
        }
    }
}

/// Redraw only what changed. Uses dirty rects for drag, full redraw otherwise.
/// Called from hlt_loop after each HLT wake.
pub fn redraw_if_needed() {
    // Step 0: Drain queued input events (try_lock to avoid deadlock in ISR context).
    while let Some(ev) = crate::gui::event_bus::try_pop_event() {
        match ev {
            crate::gui::event_bus::GuiInputEvent::MouseState { x, y, buttons } => {
                on_mouse_update(x, y, buttons);
                if let Some(ref guard) = DESKTOP.try_lock() {
                    if let Some(ref desktop) = **guard {
                        if desktop.active_user_window_contains(x, y) {
                            if let Some(slot) = desktop.active_user_window_slot() {
                                if let Some(ref w) = desktop.wm.windows[slot] {
                                    crate::gui::event_bus::push_user_mouse_event(w.id as u32, x, y, buttons);
                                }
                            }
                        }
                    }
                }
            }
            crate::gui::event_bus::GuiInputEvent::KeyPress { key } => {
                on_key_press(key);
                let key_code: u16 = match key {
                    crate::gui::event::Key::Char(c) => c as u16,
                    crate::gui::event::Key::Backspace => 0x0008,
                    crate::gui::event::Key::Enter => 0x000D,
                    crate::gui::event::Key::Tab => 0x0009,
                    crate::gui::event::Key::Escape => 0x001B,
                    crate::gui::event::Key::ArrowUp => 0x0101,
                    crate::gui::event::Key::ArrowDown => 0x0102,
                    crate::gui::event::Key::ArrowLeft => 0x0103,
                    crate::gui::event::Key::ArrowRight => 0x0104,
                    crate::gui::event::Key::Delete => 0x007F,
                    crate::gui::event::Key::Home => 0x0105,
                    crate::gui::event::Key::End => 0x0106,
                    crate::gui::event::Key::PageUp => 0x0107,
                    crate::gui::event::Key::PageDown => 0x0108,
                };
                if let Some(ref guard) = DESKTOP.try_lock() {
                    if let Some(ref desktop) = **guard {
                        if let Some(slot) = desktop.active_user_window_slot() {
                            if let Some(ref w) = desktop.wm.windows[slot] {
                                crate::gui::event_bus::push_user_key_event(w.id as u32, key_code);
                            }
                        }
                    }
                }
            }
        }
    }

    // Step 1: Tick animations (may generate new dirty rects)
    {
        if let Some(ref mut guard) = DESKTOP.try_lock() {
            if let Some(ref mut desktop) = **guard {
                desktop.tick_animations();
            }
        }
    }

    // Step 2: Quick check without holding FB lock
    let (needs_full, has_dirty) = {
        match DESKTOP.try_lock() {
            Some(guard) => match guard.as_ref() {
                Some(d) => (d.needs_full_redraw, d.dirty_count > 0),
                None => (false, false),
            },
            None => (false, false),
        }
    };

    if !needs_full && !has_dirty { return; }

    // Step 3: Acquire both locks and render
    let mut fb_guard = match FRAMEBUFFER.try_lock() {
        Some(g) => g,
        None => return,
    };
    let fb = match fb_guard.as_mut() {
        Some(fb) => fb,
        None => return,
    };
    let mut desk_guard = match DESKTOP.try_lock() {
        Some(g) => g,
        None => return,
    };
    if let Some(ref mut desktop) = *desk_guard {
        if desktop.needs_full_redraw {
            desktop.draw_full(fb);
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
            fb.flush_rect(desktop.mouse_x, desktop.mouse_y, 14, 20);
        } else if desktop.dirty_count > 0 {
            // Always include cursor area in partial redraw to prevent ghost/duplication
            // artifacts: guarantees draw_partial clears cursor pixels before we re-save
            // background under redraw_cursor_on.
            let cx = desktop.mouse_x;
            let cy = desktop.mouse_y;
            desktop.mark_dirty(Rect::new(cx, cy, 14, 20));
            desktop.draw_partial(fb);
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
            fb.flush_rect(desktop.mouse_x, desktop.mouse_y, 14, 20);
        }
    }
}

/// Called from mouse IRQ handler to update the desktop with new mouse state.
pub fn on_mouse_update(x: usize, y: usize, buttons: u8) {
    let mut guard = match DESKTOP.try_lock() {
        Some(g) => g,
        None => return,
    };
    if let Some(ref mut desktop) = *guard {
        desktop.on_mouse(x, y, buttons);
    }
}

/// Called from keyboard IRQ handler to dispatch key events to the active window.
pub fn on_key_press(key: crate::gui::event::Key) {
    let mut guard = match DESKTOP.try_lock() {
        Some(g) => g,
        None => return,
    };
    if let Some(ref mut desktop) = *guard {
        desktop.on_key(key);
    }
}

/// Create a user window from syscall path.
pub fn create_user_window(x: usize, y: usize, w: usize, h: usize, title: &str) -> Option<usize> {
    let mut guard = DESKTOP.lock();
    let desktop = guard.as_mut()?;
    let mut win = Window::new(0, x, y, w.max(160), h.max(100), title);
    win.set_user_owned(true);
    win.set_content("Userland window connected via syscall");
    let id = desktop.add_window(win);
    crate::gui::event_bus::push_user_window_created(id as u32);
    Some(id)
}

/// Set textual content of a window by id.
pub fn set_window_text(win_id: usize, text: &str) -> bool {
    let mut guard = DESKTOP.lock();
    let desktop = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };

    for i in 0..desktop.wm.count {
        let slot = desktop.wm.order[i];
        let mut dirty_bounds = None;
        if let Some(ref mut w) = desktop.wm.windows[slot] {
            if w.id == win_id {
                w.set_content(text);
                dirty_bounds = Some(w.bounds_with_shadow());
            }
        }
        if let Some(bounds) = dirty_bounds {
            desktop.mark_dirty(bounds);
            return true;
        }
    }
    false
}

/// Request full desktop redraw from external callers (e.g. syscalls).
pub fn request_redraw() {
    let mut guard = DESKTOP.lock();
    if let Some(ref mut desktop) = *guard {
        desktop.request_redraw();
    }
}

/// Set a rectangle primitive on a target user window.
pub fn set_window_rect(win_id: usize, x: usize, y: usize, w: usize, h: usize, rgb: u32) -> bool {
    let mut guard = DESKTOP.lock();
    let desktop = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };

    for i in 0..desktop.wm.count {
        let slot = desktop.wm.order[i];
        let mut dirty_bounds = None;
        if let Some(ref mut win) = desktop.wm.windows[slot] {
            if win.id == win_id {
                win.set_user_rect(x, y, w, h, rgb);
                dirty_bounds = Some(win.bounds_with_shadow());
            }
        }
        if let Some(bounds) = dirty_bounds {
            desktop.mark_dirty(bounds);
            return true;
        }
    }
    false
}

/// Injects a high-performance raw application pixel buffer into the window context.
pub fn set_window_buffer(win_id: usize, x: usize, y: usize, w: usize, h: usize, buf: alloc::vec::Vec<u8>) -> bool {
    let mut guard = DESKTOP.lock();
    let desktop = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };

    for i in 0..desktop.wm.count {
        let slot = desktop.wm.order[i];
        if let Some(ref mut win) = desktop.wm.windows[slot] {
            if win.id == win_id {
                win.set_user_buffer(x, y, w, h, buf); // Ownership transferred
                let bounds = win.bounds_with_shadow();
                desktop.mark_dirty(bounds);
                return true;
            }
        }
    }
    false
}

/// Destroy a window by id.
pub fn destroy_window(win_id: usize) -> bool {
    let mut guard = DESKTOP.lock();
    let desktop = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };
    if let Some(bounds) = desktop.wm.remove_window_by_id(win_id) {
        desktop.mark_dirty(bounds);
        desktop.request_redraw();
        crate::gui::event_bus::push_user_window_destroyed(win_id as u32);
        true
    } else {
        false
    }
}

/// Focus an existing window by id.
pub fn focus_window(win_id: usize) -> bool {
    let mut guard = DESKTOP.lock();
    let desktop = match guard.as_mut() {
        Some(d) => d,
        None => return false,
    };

    let old_active = desktop.wm.active;
    let slot = match desktop.wm.find_slot_by_id(win_id) {
        Some(s) => s,
        None => return false,
    };

    if desktop.wm.bring_to_front(slot) || old_active != desktop.wm.active {
        if let Some(old_idx) = old_active {
            if let Some(ref w) = desktop.wm.windows[old_idx] {
                desktop.mark_dirty(w.bounds_with_shadow());
            }
        }
        if let Some(new_idx) = desktop.wm.active {
            if let Some(ref w) = desktop.wm.windows[new_idx] {
                desktop.mark_dirty(w.bounds_with_shadow());
            }
        }
        desktop.request_redraw();
    }

    true
}
