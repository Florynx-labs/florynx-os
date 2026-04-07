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
            }
        }
        if let Some(ref mut w) = self.windows[slot] {
            w.active = true;
        }
        self.active = Some(slot);
        changed
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
}

// ---------------------------------------------------------------------------
// Desktop
// ---------------------------------------------------------------------------

const MAX_DIRTY: usize = 8;

pub struct Desktop {
    wm: WindowManager,
    dock: Dock,
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
            needs_full_redraw: true,
            bg_cache: Vec::new(),
            bg_cached: false,
            dirty: [Rect::new(0, 0, 0, 0); MAX_DIRTY],
            dirty_count: 0,
        }
    }

    /// Add a window to the desktop.
    pub fn add_window(&mut self, win: Window) -> usize {
        let id = self.wm.add_window(win);
        self.needs_full_redraw = true;
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

    /// Full draw: background + all windows + dock. Used for first render or major changes.
    pub fn draw_full(&mut self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;

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
            // Blit full cached background
            let full = Rect::new(0, 0, self.screen_w, self.screen_h);
            self.blit_bg_rect(fb, &full);
        }

        self.wm.draw(fb);
        self.dock.draw(fb, self.screen_w, self.screen_h);

        self.needs_full_redraw = false;
        self.dirty_count = 0;
    }

    /// Partial draw: only repaint dirty rectangles. Much faster than full redraw.
    fn draw_partial(&mut self, fb: &mut FramebufferManager) {
        if self.dirty_count == 0 { return; }

        // 1. Blit background cache over each dirty rect (erase old content)
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

        self.dirty_count = 0;
    }

    /// Process keyboard input from the PS/2 keyboard driver.
    pub fn on_key(&mut self, key: crate::gui::event::Key) {
        use crate::gui::event::Event;
        
        // Dispatch key event to the active window
        if let Some(active_idx) = self.wm.active {
            if let Some(ref mut win) = self.wm.windows[active_idx] {
                let event = Event::KeyPress { key };
                if win.handle_event(&event, self.screen_w, self.screen_h) {
                    // Window consumed the key event, mark it dirty for redraw
                    let bounds = win.bounds_with_shadow();
                    self.mark_dirty(bounds);
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
                // Dock icon clicked - create window based on icon
                self.on_dock_click(icon_idx);
                self.needs_full_redraw = true;
                return;
            }
            
            // Then check windows
            self.wm.handle_event(&ev, self.screen_w, self.screen_h);
        }

        if !left_now && left_was {
            let ev = Event::MouseUp { x, y, button: MouseButton::Left };
            self.wm.handle_event(&ev, self.screen_w, self.screen_h);
        }

        if x != self.mouse_x || y != self.mouse_y {
            self.mouse_x = x;
            self.mouse_y = y;
            let ev = Event::MouseMove { x, y };

            if self.wm.any_dragging() {
                // Save old window bounds BEFORE the move
                let old_bounds = self.wm.dragged_bounds_with_shadow();

                // Process the drag (moves the window)
                self.wm.handle_event(&ev, self.screen_w, self.screen_h);

                // Mark old position + new position as dirty
                if let Some(old_r) = old_bounds {
                    self.mark_dirty(old_r);
                }
                if let Some(new_r) = self.wm.dragged_bounds_with_shadow() {
                    self.mark_dirty(new_r);
                }
            } else {
                // Update dock hover state
                self.dock.handle_event(&ev, self.screen_w, self.screen_h);
            }
        }
    }

    pub fn needs_redraw(&self) -> bool {
        self.needs_full_redraw || self.dirty_count > 0
    }

    pub fn request_redraw(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Handle dock icon click - create appropriate window
    fn on_dock_click(&mut self, icon_idx: usize) {
        use crate::gui::window::Window;
        
        let (x, y) = (100 + icon_idx * 30, 100 + icon_idx * 30);
        
        let win = match icon_idx {
            0 => {
                // Files icon
                let mut w = Window::new(0, x, y, 500, 400, "Files");
                w.set_content("File Manager\n\nBrowse your files here.\n\n(VFS not yet implemented)");
                w
            }
            1 => {
                // Terminal icon
                let mut w = Window::new(0, x, y, 600, 400, "Terminal");
                w.set_content("Florynx Terminal\n\n$ Welcome to FlorynxOS\n$ Type commands here\n\n(Shell not yet implemented)");
                w
            }
            2 => {
                // Settings icon
                let mut w = Window::new(0, x, y, 450, 350, "Settings");
                w.set_content("System Settings\n\nConfigure your system:\n- Display\n- Keyboard\n- Mouse\n- Network\n\n(Settings panel coming soon)");
                w
            }
            3 => {
                // Monitor icon
                let mut w = Window::new(0, x, y, 500, 400, "System Monitor");
                w.set_content("System Monitor\n\nCPU: AMD64\nMemory: 4 MiB heap\nUptime: Running\n\n(Real-time stats coming soon)");
                w
            }
            4 => {
                // Notes/Document icon
                let mut w = Window::new(0, x, y, 500, 400, "Notes");
                w.set_content("Welcome to Notes!\n\nType your notes here.\nPress Enter for new lines.\nBackspace to delete.\n\nThis is a simple text editor.");
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

/// Draw the desktop (called once at startup).
pub fn draw() {
    if let Some(ref mut fb) = *FRAMEBUFFER.lock() {
        if let Some(ref mut desktop) = *DESKTOP.lock() {
            desktop.draw_full(fb);
            renderer::redraw_cursor_on(fb, desktop.mouse_x, desktop.mouse_y);
        }
    }
}

/// Redraw only what changed. Uses dirty rects for drag, full redraw otherwise.
/// Called from hlt_loop after each HLT wake.
pub fn redraw_if_needed() {
    // Quick check without holding FB lock
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
        } else if desktop.dirty_count > 0 {
            desktop.draw_partial(fb);
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
