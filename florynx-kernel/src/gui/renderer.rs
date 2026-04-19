// =============================================================================
// Florynx Kernel — GUI Renderer
// =============================================================================
// Central rendering abstraction for the Florynx GUI. All drawing goes through
// this module. Provides primitives: rect, rounded rect, gradient, text,
// shadow, line, and cursor management.
// =============================================================================

pub use crate::drivers::display::framebuffer::{FRAMEBUFFER, FramebufferManager};
use ab_glyph::{Font, FontRef, PxScale, ScaleFont, point};
use lazy_static::lazy_static;

static ROBOTO_BYTES: &[u8] = include_bytes!("assets/fonts/Roboto/static/Roboto-Regular.ttf");

lazy_static! {
    pub static ref ROBOTO_FONT: FontRef<'static> = FontRef::try_from_slice(ROBOTO_BYTES).expect("Failed to load Roboto font");
}

// ---------------------------------------------------------------------------
// Color
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }
    pub const WHITE: Color     = Color::rgb(255, 255, 255);
    pub const BLACK: Color     = Color::rgb(0, 0, 0);
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);
}

// ---------------------------------------------------------------------------
// Embedded 8x8 bitmap font — ASCII 32..=126 (95 glyphs)
// Shared by renderer::draw_text and gui::console
// ---------------------------------------------------------------------------

pub const FONT_W: usize = 8;
pub const FONT_H: usize = 8;

#[rustfmt::skip]
pub static FONT: [[u8; 8]; 95] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 0x20 ' '
    [0x18,0x3C,0x3C,0x18,0x18,0x00,0x18,0x00], // 0x21 '!'
    [0x36,0x36,0x00,0x00,0x00,0x00,0x00,0x00], // 0x22 '"'
    [0x36,0x36,0x7F,0x36,0x7F,0x36,0x36,0x00], // 0x23 '#'
    [0x0C,0x3E,0x03,0x1E,0x30,0x1F,0x0C,0x00], // 0x24 '$'
    [0x00,0x63,0x33,0x18,0x0C,0x66,0x63,0x00], // 0x25 '%'
    [0x1C,0x36,0x1C,0x6E,0x3B,0x33,0x6E,0x00], // 0x26 '&'
    [0x06,0x06,0x03,0x00,0x00,0x00,0x00,0x00], // 0x27 '''
    [0x18,0x0C,0x06,0x06,0x06,0x0C,0x18,0x00], // 0x28 '('
    [0x06,0x0C,0x18,0x18,0x18,0x0C,0x06,0x00], // 0x29 ')'
    [0x00,0x66,0x3C,0xFF,0x3C,0x66,0x00,0x00], // 0x2A '*'
    [0x00,0x0C,0x0C,0x3F,0x0C,0x0C,0x00,0x00], // 0x2B '+'
    [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C,0x06], // 0x2C ','
    [0x00,0x00,0x00,0x3F,0x00,0x00,0x00,0x00], // 0x2D '-'
    [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C,0x00], // 0x2E '.'
    [0x60,0x30,0x18,0x0C,0x06,0x03,0x01,0x00], // 0x2F '/'
    [0x3E,0x63,0x73,0x7B,0x6F,0x67,0x3E,0x00], // 0x30 '0'
    [0x0C,0x0E,0x0C,0x0C,0x0C,0x0C,0x3F,0x00], // 0x31 '1'
    [0x1E,0x33,0x30,0x1C,0x06,0x33,0x3F,0x00], // 0x32 '2'
    [0x1E,0x33,0x30,0x1C,0x30,0x33,0x1E,0x00], // 0x33 '3'
    [0x38,0x3C,0x36,0x33,0x7F,0x30,0x78,0x00], // 0x34 '4'
    [0x3F,0x03,0x1F,0x30,0x30,0x33,0x1E,0x00], // 0x35 '5'
    [0x1C,0x06,0x03,0x1F,0x33,0x33,0x1E,0x00], // 0x36 '6'
    [0x3F,0x33,0x30,0x18,0x0C,0x0C,0x0C,0x00], // 0x37 '7'
    [0x1E,0x33,0x33,0x1E,0x33,0x33,0x1E,0x00], // 0x38 '8'
    [0x1E,0x33,0x33,0x3E,0x30,0x18,0x0E,0x00], // 0x39 '9'
    [0x00,0x0C,0x0C,0x00,0x00,0x0C,0x0C,0x00], // 0x3A ':'
    [0x00,0x0C,0x0C,0x00,0x00,0x0C,0x0C,0x06], // 0x3B ';'
    [0x18,0x0C,0x06,0x03,0x06,0x0C,0x18,0x00], // 0x3C '<'
    [0x00,0x00,0x3F,0x00,0x00,0x3F,0x00,0x00], // 0x3D '='
    [0x06,0x0C,0x18,0x30,0x18,0x0C,0x06,0x00], // 0x3E '>'
    [0x1E,0x33,0x30,0x18,0x0C,0x00,0x0C,0x00], // 0x3F '?'
    [0x3E,0x63,0x7B,0x7B,0x7B,0x03,0x1E,0x00], // 0x40 '@'
    [0x0C,0x1E,0x33,0x33,0x3F,0x33,0x33,0x00], // 0x41 'A'
    [0x3F,0x66,0x66,0x3E,0x66,0x66,0x3F,0x00], // 0x42 'B'
    [0x3C,0x66,0x03,0x03,0x03,0x66,0x3C,0x00], // 0x43 'C'
    [0x1F,0x36,0x66,0x66,0x66,0x36,0x1F,0x00], // 0x44 'D'
    [0x7F,0x46,0x16,0x1E,0x16,0x46,0x7F,0x00], // 0x45 'E'
    [0x7F,0x46,0x16,0x1E,0x16,0x06,0x0F,0x00], // 0x46 'F'
    [0x3C,0x66,0x03,0x03,0x73,0x66,0x7C,0x00], // 0x47 'G'
    [0x33,0x33,0x33,0x3F,0x33,0x33,0x33,0x00], // 0x48 'H'
    [0x1E,0x0C,0x0C,0x0C,0x0C,0x0C,0x1E,0x00], // 0x49 'I'
    [0x78,0x30,0x30,0x30,0x33,0x33,0x1E,0x00], // 0x4A 'J'
    [0x67,0x66,0x36,0x1E,0x36,0x66,0x67,0x00], // 0x4B 'K'
    [0x0F,0x06,0x06,0x06,0x46,0x66,0x7F,0x00], // 0x4C 'L'
    [0x63,0x77,0x7F,0x7F,0x6B,0x63,0x63,0x00], // 0x4D 'M'
    [0x63,0x67,0x6F,0x7B,0x73,0x63,0x63,0x00], // 0x4E 'N'
    [0x1C,0x36,0x63,0x63,0x63,0x36,0x1C,0x00], // 0x4F 'O'
    [0x3F,0x66,0x66,0x3E,0x06,0x06,0x0F,0x00], // 0x50 'P'
    [0x1E,0x33,0x33,0x33,0x3B,0x1E,0x38,0x00], // 0x51 'Q'
    [0x3F,0x66,0x66,0x3E,0x36,0x66,0x67,0x00], // 0x52 'R'
    [0x1E,0x33,0x07,0x0E,0x38,0x33,0x1E,0x00], // 0x53 'S'
    [0x3F,0x2D,0x0C,0x0C,0x0C,0x0C,0x1E,0x00], // 0x54 'T'
    [0x33,0x33,0x33,0x33,0x33,0x33,0x3F,0x00], // 0x55 'U'
    [0x33,0x33,0x33,0x33,0x33,0x1E,0x0C,0x00], // 0x56 'V'
    [0x63,0x63,0x63,0x6B,0x7F,0x77,0x63,0x00], // 0x57 'W'
    [0x63,0x63,0x36,0x1C,0x1C,0x36,0x63,0x00], // 0x58 'X'
    [0x33,0x33,0x33,0x1E,0x0C,0x0C,0x1E,0x00], // 0x59 'Y'
    [0x7F,0x63,0x31,0x18,0x4C,0x66,0x7F,0x00], // 0x5A 'Z'
    [0x1E,0x06,0x06,0x06,0x06,0x06,0x1E,0x00], // 0x5B '['
    [0x03,0x06,0x0C,0x18,0x30,0x60,0x40,0x00], // 0x5C '\'
    [0x1E,0x18,0x18,0x18,0x18,0x18,0x1E,0x00], // 0x5D ']'
    [0x08,0x1C,0x36,0x63,0x00,0x00,0x00,0x00], // 0x5E '^'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xFF], // 0x5F '_'
    [0x0C,0x0C,0x18,0x00,0x00,0x00,0x00,0x00], // 0x60 '`'
    [0x00,0x00,0x1E,0x30,0x3E,0x33,0x6E,0x00], // 0x61 'a'
    [0x07,0x06,0x06,0x3E,0x66,0x66,0x3B,0x00], // 0x62 'b'
    [0x00,0x00,0x1E,0x33,0x03,0x33,0x1E,0x00], // 0x63 'c'
    [0x38,0x30,0x30,0x3E,0x33,0x33,0x6E,0x00], // 0x64 'd'
    [0x00,0x00,0x1E,0x33,0x3F,0x03,0x1E,0x00], // 0x65 'e'
    [0x1C,0x36,0x06,0x0F,0x06,0x06,0x0F,0x00], // 0x66 'f'
    [0x00,0x00,0x6E,0x33,0x33,0x3E,0x30,0x1F], // 0x67 'g'
    [0x07,0x06,0x36,0x6E,0x66,0x66,0x67,0x00], // 0x68 'h'
    [0x0C,0x00,0x0E,0x0C,0x0C,0x0C,0x1E,0x00], // 0x69 'i'
    [0x30,0x00,0x30,0x30,0x30,0x33,0x33,0x1E], // 0x6A 'j'
    [0x07,0x06,0x66,0x36,0x1E,0x36,0x67,0x00], // 0x6B 'k'
    [0x0E,0x0C,0x0C,0x0C,0x0C,0x0C,0x1E,0x00], // 0x6C 'l'
    [0x00,0x00,0x33,0x7F,0x7F,0x6B,0x63,0x00], // 0x6D 'm'
    [0x00,0x00,0x1F,0x33,0x33,0x33,0x33,0x00], // 0x6E 'n'
    [0x00,0x00,0x1E,0x33,0x33,0x33,0x1E,0x00], // 0x6F 'o'
    [0x00,0x00,0x3B,0x66,0x66,0x3E,0x06,0x0F], // 0x70 'p'
    [0x00,0x00,0x6E,0x33,0x33,0x3E,0x30,0x78], // 0x71 'q'
    [0x00,0x00,0x3B,0x6E,0x66,0x06,0x0F,0x00], // 0x72 'r'
    [0x00,0x00,0x3E,0x03,0x1E,0x30,0x1F,0x00], // 0x73 's'
    [0x08,0x0C,0x3E,0x0C,0x0C,0x2C,0x18,0x00], // 0x74 't'
    [0x00,0x00,0x33,0x33,0x33,0x33,0x6E,0x00], // 0x75 'u'
    [0x00,0x00,0x33,0x33,0x33,0x1E,0x0C,0x00], // 0x76 'v'
    [0x00,0x00,0x63,0x6B,0x7F,0x7F,0x36,0x00], // 0x77 'w'
    [0x00,0x00,0x63,0x36,0x1C,0x36,0x63,0x00], // 0x78 'x'
    [0x00,0x00,0x33,0x33,0x33,0x3E,0x30,0x1F], // 0x79 'y'
    [0x00,0x00,0x3F,0x19,0x0C,0x26,0x3F,0x00], // 0x7A 'z'
    [0x38,0x0C,0x0C,0x07,0x0C,0x0C,0x38,0x00], // 0x7B '{'
    [0x18,0x18,0x18,0x00,0x18,0x18,0x18,0x00], // 0x7C '|'
    [0x07,0x0C,0x0C,0x38,0x0C,0x0C,0x07,0x00], // 0x7D '}'
    [0x6E,0x3B,0x00,0x00,0x00,0x00,0x00,0x00], // 0x7E '~'
];

// ---------------------------------------------------------------------------
// Framebuffer access helper
// ---------------------------------------------------------------------------

/// Executes a drawing closure with a locked framebuffer.
pub fn with_fb<F>(f: F)
where
    F: FnOnce(&mut FramebufferManager),
{
    if let Some(ref mut fb) = *FRAMEBUFFER.lock() {
        f(fb);
    }
}

// ---------------------------------------------------------------------------
// Core drawing primitives
// ---------------------------------------------------------------------------

/// Fill entire screen with a solid color.
pub fn fill_screen(fb: &mut FramebufferManager, color: Color) {
    let (w, h) = fb.dimensions();
    draw_rect(fb, 0, 0, w, h, color);
}

/// Draw a gradient with subtle noise overlay for a modern look.
/// Uses a simple LCG for pseudo-random noise generation.
pub fn draw_gradient_with_noise(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, 
                                 top: Color, bottom: Color, noise_strength: u8) {
    if w == 0 || h == 0 { return; }
    
    let mut rng_state = 0x12345678u32;
    
    for dy in 0..h {
        let t = (dy * 256) / h;
        let r = ((top.r as usize * (256 - t) + bottom.r as usize * t) / 256) as u8;
        let g = ((top.g as usize * (256 - t) + bottom.g as usize * t) / 256) as u8;
        let b = ((top.b as usize * (256 - t) + bottom.b as usize * t) / 256) as u8;
        
        for dx in 0..w {
            // Simple LCG: next = (a * prev + c) mod m
            rng_state = rng_state.wrapping_mul(1103515245).wrapping_add(12345);
            let noise = ((rng_state >> 16) & 0xFF) as i16 - 128;
            let noise_val = (noise * noise_strength as i16) / 128;
            
            let nr = (r as i16 + noise_val).clamp(0, 255) as u8;
            let ng = (g as i16 + noise_val).clamp(0, 255) as u8;
            let nb = (b as i16 + noise_val).clamp(0, 255) as u8;
            
            fb.set_pixel(x + dx, y + dy, nr, ng, nb);
        }
    }
}

/// Integer square root approximation (Newton's method).
fn isqrt(n: usize) -> usize {
    if n == 0 { return 0; }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}

/// Draw a vignette effect (darkening at edges).
pub fn draw_vignette(fb: &mut FramebufferManager, strength: u8) {
    let (w, h) = fb.dimensions();
    if w == 0 || h == 0 { return; }
    
    let cx = w / 2;
    let cy = h / 2;
    let max_dist = isqrt(cx * cx + cy * cy);
    
    for py in 0..h {
        for px in 0..w {
            let dx = (px as i32 - cx as i32).abs() as usize;
            let dy = (py as i32 - cy as i32).abs() as usize;
            let dist = isqrt(dx * dx + dy * dy);
            
            if dist > max_dist / 3 {
                let fade = ((dist - max_dist / 3) * 255) / (max_dist * 2 / 3);
                let fade = fade.min(255);
                let darken = (fade * strength as usize / 255).min(255) as u8;
                
                let (r, g, b) = fb.get_pixel(px, py);
                fb.set_pixel(px, py, 
                    r.saturating_sub(darken),
                    g.saturating_sub(darken),
                    b.saturating_sub(darken));
            }
        }
    }
}

/// Draws a solid rectangle (with alpha blending support).
pub fn draw_rect(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, color: Color) {
    let (sw, sh) = fb.dimensions();
    let x1 = x.min(sw);
    let y1 = y.min(sh);
    let x2 = (x + w).min(sw);
    let y2 = (y + h).min(sh);
    let a = color.a as u16;

    for py in y1..y2 {
        for px in x1..x2 {
            if a == 255 {
                fb.set_pixel(px, py, color.r, color.g, color.b);
            } else if a > 0 {
                let (br, bg, bb) = fb.get_pixel(px, py);
                let inv_a = 255 - a;
                let nr = ((color.r as u16 * a + br as u16 * inv_a) / 255) as u8;
                let ng = ((color.g as u16 * a + bg as u16 * inv_a) / 255) as u8;
                let nb = ((color.b as u16 * a + bb as u16 * inv_a) / 255) as u8;
                fb.set_pixel(px, py, nr, ng, nb);
            }
        }
    }
}

/// Draws a vertical gradient rectangle.
pub fn draw_gradient_rect(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, top: Color, bot: Color) {
    if h == 0 { return; }
    for dy in 0..h {
        let r = (top.r as i32 + (bot.r as i32 - top.r as i32) * dy as i32 / h as i32) as u8;
        let g = (top.g as i32 + (bot.g as i32 - top.g as i32) * dy as i32 / h as i32) as u8;
        let b = (top.b as i32 + (bot.b as i32 - top.b as i32) * dy as i32 / h as i32) as u8;
        for dx in 0..w {
            fb.set_pixel(x + dx, y + dy, r, g, b);
        }
    }
}

/// Draws a horizontal line.
pub fn draw_hline(fb: &mut FramebufferManager, x: usize, y: usize, len: usize, color: Color) {
    let a = color.a as u16;
    for dx in 0..len {
        if a == 255 {
            fb.set_pixel(x + dx, y, color.r, color.g, color.b);
        } else if a > 0 {
            let (br, bg, bb) = fb.get_pixel(x + dx, y);
            let inv_a = 255 - a;
            let nr = ((color.r as u16 * a + br as u16 * inv_a) / 255) as u8;
            let ng = ((color.g as u16 * a + bg as u16 * inv_a) / 255) as u8;
            let nb = ((color.b as u16 * a + bb as u16 * inv_a) / 255) as u8;
            fb.set_pixel(x + dx, y, nr, ng, nb);
        }
    }
}

/// Draws a vertical line.
pub fn draw_vline(fb: &mut FramebufferManager, x: usize, y: usize, len: usize, color: Color) {
    let a = color.a as u16;
    for dy in 0..len {
        if a == 255 {
            fb.set_pixel(x, y + dy, color.r, color.g, color.b);
        } else if a > 0 {
            let (br, bg, bb) = fb.get_pixel(x, y + dy);
            let inv_a = 255 - a;
            let nr = ((color.r as u16 * a + br as u16 * inv_a) / 255) as u8;
            let ng = ((color.g as u16 * a + bg as u16 * inv_a) / 255) as u8;
            let nb = ((color.b as u16 * a + bb as u16 * inv_a) / 255) as u8;
            fb.set_pixel(x, y + dy, nr, ng, nb);
        }
    }
}

// ---------------------------------------------------------------------------
// Rounded rectangle
// ---------------------------------------------------------------------------

/// Draws a filled rounded rectangle with alpha blending support.
pub fn draw_rounded_rect(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, r: usize, color: Color) {
    if w == 0 || h == 0 || color.a == 0 { return; }
    let r = r.min(w / 2).min(h / 2);
    let r2 = (r * r) as i32;
    let a = color.a as u16;

    // Helper closure for blending
    let plot = |fb: &mut FramebufferManager, px: usize, py: usize| {
        if a == 255 {
            fb.set_pixel(px, py, color.r, color.g, color.b);
        } else {
            let (br, bg, bb) = fb.get_pixel(px, py);
            let inv_a = 255 - a;
            let nr = ((color.r as u16 * a + br as u16 * inv_a) / 255) as u8;
            let ng = ((color.g as u16 * a + bg as u16 * inv_a) / 255) as u8;
            let nb = ((color.b as u16 * a + bb as u16 * inv_a) / 255) as u8;
            fb.set_pixel(px, py, nr, ng, nb);
        }
    };

    // Center band (full width, between corner rows)
    draw_rect(fb, x, y + r, w, h.saturating_sub(2 * r), color);

    // Top and bottom bands (between corners horizontally)
    draw_rect(fb, x + r, y, w.saturating_sub(2 * r), r, color);
    draw_rect(fb, x + r, y + h - r, w.saturating_sub(2 * r), r, color);

    // Four rounded corners
    for dy in 0..r {
        for dx in 0..r {
            let dist = (r as i32 - 1 - dx as i32) * (r as i32 - 1 - dx as i32)
                     + (r as i32 - 1 - dy as i32) * (r as i32 - 1 - dy as i32);
            if dist <= r2 {
                plot(fb, x + dx, y + dy);                       // Top-left
                plot(fb, x + w - 1 - dx, y + dy);               // Top-right
                plot(fb, x + dx, y + h - 1 - dy);               // Bottom-left
                plot(fb, x + w - 1 - dx, y + h - 1 - dy);       // Bottom-right
            }
        }
    }
}

/// Draws a 1px rounded border (outline only).
pub fn draw_rounded_border(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, r: usize, color: Color) {
    if w < 2 || h < 2 || color.a == 0 { return; }
    let r = r.min(w / 2).min(h / 2);
    let a = color.a as u16;

    // Helper closure for blending
    let plot = |fb: &mut FramebufferManager, px: usize, py: usize| {
        if a == 255 {
            fb.set_pixel(px, py, color.r, color.g, color.b);
        } else {
            let (br, bg, bb) = fb.get_pixel(px, py);
            let inv_a = 255 - a;
            let nr = ((color.r as u16 * a + br as u16 * inv_a) / 255) as u8;
            let ng = ((color.g as u16 * a + bg as u16 * inv_a) / 255) as u8;
            let nb = ((color.b as u16 * a + bb as u16 * inv_a) / 255) as u8;
            fb.set_pixel(px, py, nr, ng, nb);
        }
    };

    // Straight edges
    draw_hline(fb, x + r, y, w.saturating_sub(2 * r), color);
    draw_hline(fb, x + r, y + h - 1, w.saturating_sub(2 * r), color);
    draw_vline(fb, x, y + r, h.saturating_sub(2 * r), color);
    draw_vline(fb, x + w - 1, y + r, h.saturating_sub(2 * r), color);

    // Corner arcs (Bresenham-ish)
    if r > 0 {
        let ri = r as i32;
        let mut cx: i32 = 0;
        let mut cy: i32 = ri;
        let mut d: i32 = 1 - ri;
        while cx <= cy {
            plot(fb, x + r - cx as usize, y + r - cy as usize);
            plot(fb, x + r - cy as usize, y + r - cx as usize);
            plot(fb, x + w - 1 - r + cx as usize, y + r - cy as usize);
            plot(fb, x + w - 1 - r + cy as usize, y + r - cx as usize);
            plot(fb, x + r - cx as usize, y + h - 1 - r + cy as usize);
            plot(fb, x + r - cy as usize, y + h - 1 - r + cx as usize);
            plot(fb, x + w - 1 - r + cx as usize, y + h - 1 - r + cy as usize);
            plot(fb, x + w - 1 - r + cy as usize, y + h - 1 - r + cx as usize);
            
            cx += 1;
            if d < 0 {
                d += 2 * cx + 1;
            } else {
                cy -= 1;
                d += 2 * (cx - cy) + 1;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Shadow
// ---------------------------------------------------------------------------

/// Draws a soft shadow behind a rectangle (multiple dark layers offset down-right).
pub fn draw_shadow(fb: &mut FramebufferManager, x: usize, y: usize, w: usize, h: usize, r: usize, layers: usize, offset: usize) {
    for i in (1..=layers).rev() {
        let alpha = 30u8.saturating_sub((i as u8).saturating_mul(8));
        let shadow_color = Color::rgba(0, 0, 0, alpha);
        let sx = x + i * offset;
        let sy = y + i * offset;
        draw_rounded_rect(fb, sx, sy, w, h, r + i, shadow_color);
    }
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

/// Draw a single character at pixel position (px, py). Scale 1 = 8x8, scale 2 = 16x16.
pub fn draw_char(fb: &mut FramebufferManager, c: u8, px: usize, py: usize, color: Color, scale: usize) {
    let idx = (c as usize).wrapping_sub(32);
    let glyph = if idx < 95 { &FONT[idx] } else { &FONT[0] };
    let s = scale.max(1);

    for gy in 0..FONT_H {
        let bits = glyph[gy];
        for gx in 0..FONT_W {
            if (bits >> gx) & 1 == 1 {
                for sy in 0..s {
                    for sx in 0..s {
                        fb.set_pixel(px + gx * s + sx, py + gy * s + sy, color.r, color.g, color.b);
                    }
                }
            }
        }
    }
}

/// Draw a text string at pixel position (px, py) with a given color and scale.
pub fn draw_text(fb: &mut FramebufferManager, text: &str, px: usize, py: usize, color: Color, scale: usize) {
    let s = scale.max(1);
    let char_w = FONT_W * s;
    let mut cx = px;
    for byte in text.bytes() {
        if byte == b'\n' {
            break; // single-line draw — caller handles multiline
        }
        let c = if byte >= 0x20 && byte <= 0x7E { byte } else { b'?' };
        draw_char(fb, c, cx, py, color, s);
        cx += char_w;
    }
}

// ---------------------------------------------------------------------------
// Anti-aliased text rendering (proportional)
// ---------------------------------------------------------------------------

/// Font size selection for the AA text system.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontSize {
    /// ~8px cell height, proportional widths — used for content text, labels
    Normal,
    /// ~16px cell height (2× scaled + AA) — used for titlebar text, headings
    Title,
}

/// Draw anti-aliased proportional text using Roboto TTF.
pub fn draw_text_aa(fb: &mut FramebufferManager, text: &str, px: usize, py: usize, color: Color, size: FontSize) {
    let scale = match size {
        FontSize::Normal => PxScale::from(14.0),
        FontSize::Title => PxScale::from(18.0),
    };

    let scaled_font = ROBOTO_FONT.as_scaled(scale);
    let mut cursor_x = px as f32;
    let cursor_y = py as f32 + scaled_font.ascent();

    for c in text.chars() {
        if c == '\n' { break; }
        let glyph = scaled_font.scaled_glyph(c);
        let mut glyph = glyph;
        glyph.position = point(cursor_x, cursor_y);

        if let Some(outline) = scaled_font.outline_glyph(glyph) {
            let bounds = outline.px_bounds();
            outline.draw(|x, y, coverage| {
                let gx = x as f32 + bounds.min.x;
                let gy = y as f32 + bounds.min.y;
                
                if gx >= 0.0 && gy >= 0.0 && coverage > 0.0 {
                    let sx = gx as usize;
                    let sy = gy as usize;
                    let (sw, sh) = fb.dimensions();
                    
                    if sx < sw && sy < sh {
                        let alpha = (coverage * 255.0) as u8;
                        let (bg_r, bg_g, bg_b) = fb.get_pixel(sx, sy);
                        let (nr, ng, nb) = crate::gui::font::alpha_blend(color, bg_r, bg_g, bg_b, alpha);
                        fb.set_pixel(sx, sy, nr, ng, nb);
                    }
                }
            });
        }
        cursor_x += scaled_font.h_advance(scaled_font.glyph_id(c));
    }
}

/// Measure text width in pixels for the given font size using Roboto TTF.
pub fn measure_text_aa(text: &str, size: FontSize) -> usize {
    let scale = match size {
        FontSize::Normal => PxScale::from(14.0),
        FontSize::Title => PxScale::from(18.0),
    };
    let scaled_font = ROBOTO_FONT.as_scaled(scale);
    let mut width = 0.0f32;
    for c in text.chars() {
        width += scaled_font.h_advance(scaled_font.glyph_id(c));
    }
    width as usize
}

/// Helper to get the horizontal advance of a single character.
pub fn char_advance_aa(c: char, size: FontSize) -> usize {
    let scale = match size {
        FontSize::Normal => PxScale::from(14.0),
        FontSize::Title => PxScale::from(18.0),
    };
    let scaled_font = ROBOTO_FONT.as_scaled(scale);
    scaled_font.h_advance(scaled_font.glyph_id(c)) as usize
}

/// Draw a small filled circle (for titlebar buttons).
pub fn draw_circle(fb: &mut FramebufferManager, cx: usize, cy: usize, r: usize, color: Color) {
    let r2 = (r * r) as i32;
    for dy in 0..=(r * 2) {
        for dx in 0..=(r * 2) {
            let ddx = dx as i32 - r as i32;
            let ddy = dy as i32 - r as i32;
            if ddx * ddx + ddy * ddy <= r2 {
                fb.set_pixel(cx.wrapping_add(dx).wrapping_sub(r),
                             cy.wrapping_add(dy).wrapping_sub(r),
                             color.r, color.g, color.b);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Cursor (called from mouse IRQ — must be fast, try_lock only)
// ---------------------------------------------------------------------------

const CURSOR_W: usize = 14;
const CURSOR_H: usize = 20;

static CURSOR_STATE: spin::Mutex<CursorBackup> = spin::Mutex::new(CursorBackup::new());

struct CursorBackup {
    buf: [u8; CURSOR_W * CURSOR_H * 3],
    x: usize,
    y: usize,
    valid: bool,
}

impl CursorBackup {
    const fn new() -> Self {
        CursorBackup {
            buf: [0u8; CURSOR_W * CURSOR_H * 3],
            x: 0,
            y: 0,
            valid: false,
        }
    }
}

fn save_under_cursor(fb: &FramebufferManager, backup: &mut CursorBackup, x: usize, y: usize) {
    backup.x = x;
    backup.y = y;
    for dy in 0..CURSOR_H {
        for dx in 0..CURSOR_W {
            let (r, g, b) = fb.get_pixel(x + dx, y + dy);
            let idx = (dy * CURSOR_W + dx) * 3;
            backup.buf[idx] = r;
            backup.buf[idx + 1] = g;
            backup.buf[idx + 2] = b;
        }
    }
    backup.valid = true;
}

fn restore_under_cursor(fb: &mut FramebufferManager, backup: &CursorBackup) {
    if !backup.valid { return; }
    for dy in 0..CURSOR_H {
        for dx in 0..CURSOR_W {
            let idx = (dy * CURSOR_W + dx) * 3;
            fb.set_pixel(backup.x + dx, backup.y + dy,
                         backup.buf[idx], backup.buf[idx + 1], backup.buf[idx + 2]);
        }
    }
}

fn draw_cursor_shape(fb: &mut FramebufferManager, x: usize, y: usize) {
    // Modern arrow cursor: white fill, black border
    // Draw a proper arrow shape that fills the 14x20 bounding box
    let arrow_pixels: [(usize, usize); 20] = [
        // Arrow head pointing up-left
        (0, 0), (1, 0), (2, 0),
        (0, 1), (1, 1), (2, 1), (3, 1),
        (0, 2), (1, 2), (2, 2), (3, 2), (4, 2),
        (0, 3), (1, 3), (2, 3), (3, 3), (4, 3), (5, 3),
        (0, 4), (1, 4),
    ];
    
    // Black outline (border pixels around the arrow)
    let outline_pixels: [(usize, usize); 15] = [
        (0, 5), (1, 5), (2, 5), (3, 5), (4, 5), (5, 5), (6, 5),
        (2, 6), (3, 6), (4, 6), (5, 6), (6, 6), (7, 6),
        (4, 7), (5, 7),
    ];
    
    // Fill white interior
    for (dx, dy) in arrow_pixels {
        if x + dx < fb.width() && y + dy < fb.height() {
            fb.set_pixel(x + dx, y + dy, 255, 255, 255);
        }
    }
    
    // Draw black outline
    for (dx, dy) in outline_pixels {
        if x + dx < fb.width() && y + dy < fb.height() {
            fb.set_pixel(x + dx, y + dy, 0, 0, 0);
        }
    }
    
    // Corner pixel
    if x < fb.width() && y < fb.height() {
        fb.set_pixel(x, y, 0, 0, 0);
    }
}

/// Update cursor position — called from mouse IRQ handler.
/// Writes to back buffer then flushes old + new cursor regions to VRAM.
pub fn update_cursor(x: usize, y: usize) {
    let mut fb_guard = match FRAMEBUFFER.try_lock() {
        Some(guard) => guard,
        None => return,
    };
    let fb = match fb_guard.as_mut() {
        Some(fb) => fb,
        None => return,
    };
    let mut backup = match CURSOR_STATE.try_lock() {
        Some(guard) => guard,
        None => return,
    };

    // Restore old pixels in back buffer and flush old region
    let old_x = backup.x;
    let old_y = backup.y;
    let was_valid = backup.valid;
    
    // Clamp coordinates to screen bounds
    let new_x = x.min(fb.width().saturating_sub(CURSOR_W));
    let new_y = y.min(fb.height().saturating_sub(CURSOR_H));
    
    restore_under_cursor(fb, &backup);
    if was_valid {
        fb.flush_rect(old_x, old_y, CURSOR_W, CURSOR_H);
    }

    // Save new pixels, draw cursor in back buffer, flush new region
    save_under_cursor(fb, &mut backup, new_x, new_y);
    draw_cursor_shape(fb, new_x, new_y);
    fb.flush_rect(new_x, new_y, CURSOR_W, CURSOR_H);
}

/// Redraw cursor on an already-locked framebuffer (e.g. after a full desktop redraw).
/// Invalidates the old backup and saves fresh pixels from the current framebuffer.
pub fn redraw_cursor_on(fb: &mut FramebufferManager, x: usize, y: usize) {
    let mut backup = match CURSOR_STATE.try_lock() {
        Some(guard) => guard,
        None => return,
    };
    
    // First restore the old cursor area to prevent ghost cursors
    let _old_x = backup.x;
    let _old_y = backup.y;
    if backup.valid {
        restore_under_cursor(fb, &backup);
        // No need to flush here - caller handles framebuffer flushing
    }
    
    // Clamp coordinates to screen bounds
    let new_x = x.min(fb.width().saturating_sub(CURSOR_W));
    let new_y = y.min(fb.height().saturating_sub(CURSOR_H));
    
    // Update backup position and save new pixels
    backup.x = new_x;
    backup.y = new_y;
    save_under_cursor(fb, &mut backup, new_x, new_y);
    draw_cursor_shape(fb, new_x, new_y);
}
