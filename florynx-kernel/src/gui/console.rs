// =============================================================================
// Florynx Kernel — Framebuffer Console
// =============================================================================
// Text-mode console rendered on the BGA framebuffer using an embedded 8x8 font.
// Characters are rendered at double height (8x16 cells) for readability.
// Provides scrolling, newline handling, and fmt::Write for println! integration.
// Prepares the foundation for future userland terminal emulation.
// =============================================================================

use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};
use spin::Mutex;
use crate::drivers::display::framebuffer::{FRAMEBUFFER, FramebufferManager};
use crate::gui::renderer::{FONT, FONT_W, FONT_H};

static FB_CONSOLE_ACTIVE: AtomicBool = AtomicBool::new(true);

// ---------------------------------------------------------------------------
// Font configuration (uses shared font from renderer)
// ---------------------------------------------------------------------------

const CELL_W: usize = FONT_W;        // 8
const CELL_H: usize = FONT_H * 2;    // 16 — each font row rendered twice (double height)

// ---------------------------------------------------------------------------
// Console state
// ---------------------------------------------------------------------------

pub struct Console {
    col: usize,
    row: usize,
    cols: usize,
    rows: usize,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
}

impl Console {
    /// Create a new console sized for the given pixel dimensions.
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        Console {
            col: 0,
            row: 0,
            cols: screen_w / CELL_W,
            rows: screen_h / CELL_H,
            fg: (200, 200, 200),  // light gray
            bg: (0, 0, 0),       // black
        }
    }

    /// Draw a single glyph at (col, row) in character-grid coordinates.
    fn draw_char(&self, fb: &mut FramebufferManager, c: u8, col: usize, row: usize) {
        let idx = (c as usize).wrapping_sub(32);
        let glyph = if idx < 95 { &FONT[idx] } else { &FONT[0] }; // fallback to space

        let px = col * CELL_W;
        let py = row * CELL_H;
        let (fr, fg, fb_) = self.fg;
        let (br, bg, bb) = self.bg;

        for gy in 0..FONT_H {
            let bits = glyph[gy];
            for gx in 0..FONT_W {
                let on = (bits >> gx) & 1 == 1;
                let (r, g, b) = if on { (fr, fg, fb_) } else { (br, bg, bb) };
                // Render each font row twice for double-height characters
                fb.set_pixel(px + gx, py + gy * 2,     r, g, b);
                fb.set_pixel(px + gx, py + gy * 2 + 1, r, g, b);
            }
        }
    }

    /// Advance to the next line, scrolling if necessary.
    fn newline(&mut self, fb: &mut FramebufferManager) {
        self.col = 0;
        if self.row + 1 < self.rows {
            self.row += 1;
        } else {
            // Scroll framebuffer up by one character row
            fb.scroll_up(CELL_H);
        }
    }

    /// Write a single byte to the console.
    pub fn write_byte(&mut self, fb: &mut FramebufferManager, byte: u8) {
        match byte {
            b'\n' => self.newline(fb),
            byte => {
                if self.col >= self.cols {
                    self.newline(fb);
                }
                let c = if byte >= 0x20 && byte <= 0x7E { byte } else { b'?' };
                self.draw_char(fb, c, self.col, self.row);
                self.col += 1;
            }
        }
    }

    /// Write a string to the console.
    pub fn write_str(&mut self, fb: &mut FramebufferManager, s: &str) {
        for byte in s.bytes() {
            self.write_byte(fb, byte);
        }
    }

    /// Clear the console and reset cursor to top-left.
    pub fn clear(&mut self, fb: &mut FramebufferManager) {
        fb.clear(self.bg.0, self.bg.1, self.bg.2);
        self.col = 0;
        self.row = 0;
    }

    /// Set foreground color.
    pub fn set_fg(&mut self, r: u8, g: u8, b: u8) {
        self.fg = (r, g, b);
    }

    /// Set background color.
    pub fn set_bg(&mut self, r: u8, g: u8, b: u8) {
        self.bg = (r, g, b);
    }
}

// ---------------------------------------------------------------------------
// fmt::Write implementation for use with write!/writeln! macros
// ---------------------------------------------------------------------------

/// Wrapper that holds both a Console reference and an FB reference for fmt::Write.
pub struct ConsoleWriter<'a> {
    console: &'a mut Console,
    fb: &'a mut FramebufferManager,
}

impl<'a> fmt::Write for ConsoleWriter<'a> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.console.write_str(self.fb, s);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Global console instance
// ---------------------------------------------------------------------------

pub static CONSOLE: Mutex<Option<Console>> = Mutex::new(None);

/// Initialize the framebuffer console. Call after BGA framebuffer is ready.
pub fn init() {
    if let Some(ref mut fb) = *FRAMEBUFFER.lock() {
        let (w, h) = fb.dimensions();
        if w == 0 || h == 0 { return; }

        let mut con = Console::new(w, h);
        con.clear(fb);
        *CONSOLE.lock() = Some(con);

        crate::serial_println!("[console] framebuffer console initialized ({}x{} chars)", w / CELL_W, h / CELL_H);
    }
}

/// Disable framebuffer console output (call when desktop takes ownership).
pub fn disable() {
    FB_CONSOLE_ACTIVE.store(false, Ordering::Relaxed);
}

/// Write a formatted string to the framebuffer console.
/// Safe to call from anywhere (acquires locks internally, skips if contended).
pub fn _print(args: fmt::Arguments) {
    if !FB_CONSOLE_ACTIVE.load(Ordering::Relaxed) {
        return;
    }
    use core::fmt::Write;

    // Acquire FB lock first, then console lock
    let mut fb_guard = match FRAMEBUFFER.try_lock() {
        Some(g) => g,
        None => return, // skip if contended (e.g. from interrupt)
    };
    let fb = match fb_guard.as_mut() {
        Some(fb) => fb,
        None => return,
    };

    let mut con_guard = match CONSOLE.try_lock() {
        Some(g) => g,
        None => return,
    };
    let con = match con_guard.as_mut() {
        Some(c) => c,
        None => return,
    };

    let mut writer = ConsoleWriter { console: con, fb };
    let _ = writer.write_fmt(args);
}
