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
use crate::gui::event::{Event, MouseButton};
use crate::gui::window::Window;
use crate::gui::dock::Dock;

const MAX_WINDOWS: usize = 16;

// ---------------------------------------------------------------------------
// Window Manager
// ---------------------------------------------------------------------------

pub struct WindowManager {
    windows: [Option<Window>; MAX_WINDOWS],
    /// Z-order: indices into `windows`, front-most first.
    order: [usize; MAX_WINDOWS],
    count: usize,
    active: Option<usize>,
    next_id: usize,
}

impl WindowManager {
    const fn new() -> Self {
        const NONE: Option<Window> = None;
        WindowManager {
            windows: [NONE; MAX_WINDOWS],
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
    fn bring_to_front(&mut self, slot: usize) {
        // Find position in order
        let mut pos = None;
        for i in 0..self.count {
            if self.order[i] == slot {
                pos = Some(i);
                break;
            }
        }
        if let Some(p) = pos {
            // Shift left from front to p, then place at front
            let val = self.order[p];
            let mut i = p;
            while i > 0 {
                self.order[i] = self.order[i - 1];
                i -= 1;
            }
            self.order[0] = val;
        }

        // Update active
        if let Some(ai) = self.active {
            if let Some(ref mut w) = self.windows[ai] {
                w.active = false;
            }
        }
        if let Some(ref mut w) = self.windows[slot] {
            w.active = true;
        }
        self.active = Some(slot);
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
                self.bring_to_front(slot);
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
}

// ---------------------------------------------------------------------------
// Desktop
// ---------------------------------------------------------------------------

pub struct Desktop {
    wm: WindowManager,
    dock: Dock,
    screen_w: usize,
    screen_h: usize,
    mouse_x: usize,
    mouse_y: usize,
    prev_buttons: u8,
    needs_redraw: bool,
    /// Cached desktop background (RGB bytes, screen_w * screen_h * 3).
    /// Rendered once on first draw, then blitted on subsequent redraws.
    bg_cache: Vec<u8>,
    bg_cached: bool,
}

impl Desktop {
    fn new(screen_w: usize, screen_h: usize) -> Self {
        let mut dock = Dock::new();
        // Default dock items with embedded icons
        dock.add(&crate::gui::icons::ICON_FOLDER, Color::rgb(0, 122, 204));      // Files
        dock.add(&crate::gui::icons::ICON_TERMINAL, Color::rgb(70, 70, 80));     // Terminal
        dock.add(&crate::gui::icons::ICON_SETTINGS, Color::rgb(50, 140, 80));    // Settings
        dock.add(&crate::gui::icons::ICON_MONITOR, Color::rgb(180, 60, 60));     // Monitor
        dock.add(&crate::gui::icons::ICON_DOCUMENT, Color::rgb(60, 60, 150));    // Notes

        // Mark first item as "active"
        dock.set_active(0, true);

        Desktop {
            wm: WindowManager::new(),
            dock,
            screen_w,
            screen_h,
            mouse_x: screen_w / 2,
            mouse_y: screen_h / 2,
            prev_buttons: 0,
            needs_redraw: true,
            bg_cache: Vec::new(),
            bg_cached: false,
        }
    }

    /// Add a window to the desktop.
    pub fn add_window(&mut self, win: Window) -> usize {
        let id = self.wm.add_window(win);
        self.needs_redraw = true;
        id
    }

    /// Draw the entire desktop to the framebuffer.
    pub fn draw(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;

        // 1. Desktop background — render once, cache, then blit from cache
        if !self.bg_cached {
            // First time: render the expensive gradient + vignette
            renderer::draw_gradient_with_noise(fb, 0, 0, self.screen_w, self.screen_h,
                t.desktop_top, t.desktop_bot, 8);
            renderer::draw_vignette(fb, 40);

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
            // Fast path: blit cached background
            for y in 0..self.screen_h {
                for x in 0..self.screen_w {
                    let idx = (y * self.screen_w + x) * 3;
                    fb.set_pixel(x, y,
                        self.bg_cache[idx],
                        self.bg_cache[idx + 1],
                        self.bg_cache[idx + 2]);
                }
            }
        }

        // 2. Windows (back to front)
        self.wm.draw(fb);

        // 3. Dock (always on top of windows)
        self.dock.draw(fb, self.screen_w, self.screen_h);

        self.needs_redraw = false;
    }

    /// Process raw mouse state from the PS/2 driver.
    /// Called from the mouse IRQ path (via desktop::on_mouse_update).
    pub fn on_mouse(&mut self, x: usize, y: usize, buttons: u8) {
        let old_buttons = self.prev_buttons;
        self.prev_buttons = buttons;

        let left_now = buttons & 1 != 0;
        let left_was = old_buttons & 1 != 0;

        // Generate events
        if left_now && !left_was {
            let ev = Event::MouseDown { x, y, button: MouseButton::Left };
            // Process event but don't redraw (focus change is visual but not critical)
            self.wm.handle_event(&ev, self.screen_w, self.screen_h);
            self.dock.handle_event(&ev, self.screen_w, self.screen_h);
        }

        if !left_now && left_was {
            let ev = Event::MouseUp { x, y, button: MouseButton::Left };
            self.wm.handle_event(&ev, self.screen_w, self.screen_h);
            // No redraw needed - drag end doesn't require immediate visual update
        }

        if x != self.mouse_x || y != self.mouse_y {
            self.mouse_x = x;
            self.mouse_y = y;
            let ev = Event::MouseMove { x, y };

            if self.wm.any_dragging() {
                // Only redraw if drag actually moved the window
                self.wm.handle_event(&ev, self.screen_w, self.screen_h);
                self.needs_redraw = true;
            } else {
                // No drag - just update hover state without redraw
                self.dock.handle_event(&ev, self.screen_w, self.screen_h);
            }
        }
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_redraw
    }

    pub fn request_redraw(&mut self) {
        self.needs_redraw = true;
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
    let mut win = Window::new(0, sw / 2 - 220, sh / 2 - 170, 440, 320, "Welcome to FlorynxOS");
    win.set_content("Florynx OS v0.2\n\nBioluminescent desktop shell\nbuilt from scratch in Rust.\n\nDrag this window around!");
    desktop.add_window(win);

    *DESKTOP.lock() = Some(desktop);

    crate::serial_println!("[desktop] GUI initialized ({}x{})", sw, sh);
}

/// Draw the desktop (called once at startup and on redraw).
pub fn draw() {
    if let Some(ref mut fb) = *FRAMEBUFFER.lock() {
        if let Some(ref mut desktop) = *DESKTOP.lock() {
            desktop.draw(fb);
            // Redraw cursor on top (invalidates old backup since we just redrew everything)
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
        }
    }
}

/// Full redraw: background + windows + dock + cursor.
/// Called from hlt_loop or timer when needs_redraw is true.
pub fn redraw_if_needed() {
    // Check if redraw needed without holding FB lock
    let needs = {
        match DESKTOP.try_lock() {
            Some(guard) => match guard.as_ref() {
                Some(d) => d.needs_redraw(),
                None => false,
            },
            None => false,
        }
    };

    if needs {
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
            desktop.draw(fb);
            // Redraw cursor on top after full desktop redraw
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
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
