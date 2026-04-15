// =============================================================================
// Florynx Kernel — Anti-Aliased Font Engine
// =============================================================================
// Glyph-based text rendering with per-pixel coverage for smooth edges.
// Supports proportional widths and multiple font sizes.
// =============================================================================

use crate::gui::renderer::{FramebufferManager, Color};

// ---------------------------------------------------------------------------
// Glyph / Font Face structures
// ---------------------------------------------------------------------------

/// A single rasterized glyph with coverage data for anti-aliased rendering.
pub struct Glyph {
    pub width: u8,
    pub height: u8,
    pub advance: u8,      // horizontal advance to next glyph origin
    pub bearing_x: i8,    // x offset from glyph origin to start of bitmap
    pub bearing_y: i8,    // y offset from baseline to top of bitmap
    pub data: &'static [u8], // coverage map (width × height bytes, 0=transparent, 255=opaque)
}

/// A complete font face at a specific size.
pub struct FontFace {
    pub glyphs: &'static [Glyph],  // 95 glyphs (ASCII 32..126)
    pub line_height: u8,
    pub ascent: u8,
    pub baseline: u8,           // distance from top of cell to baseline
}

// ---------------------------------------------------------------------------
// Alpha blending
// ---------------------------------------------------------------------------

/// Alpha-blend a foreground color onto a background color.
/// `alpha` is 0..255 where 255 = fully opaque foreground.
#[inline]
pub fn alpha_blend(fg: Color, bg_r: u8, bg_g: u8, bg_b: u8, alpha: u8) -> (u8, u8, u8) {
    if alpha == 0 { return (bg_r, bg_g, bg_b); }
    if alpha == 255 { return (fg.r, fg.g, fg.b); }
    let a = alpha as u16;
    let ia = 255 - a;
    let r = ((fg.r as u16 * a + bg_r as u16 * ia) / 255) as u8;
    let g = ((fg.g as u16 * a + bg_g as u16 * ia) / 255) as u8;
    let b = ((fg.b as u16 * a + bg_b as u16 * ia) / 255) as u8;
    (r, g, b)
}

// ---------------------------------------------------------------------------
// Glyph rendering
// ---------------------------------------------------------------------------

/// Draw a single anti-aliased glyph at (px, py) using coverage blending.
pub fn draw_glyph_aa(fb: &mut FramebufferManager, glyph: &Glyph, px: usize, py: usize, color: Color) {
    let (sw, sh) = fb.dimensions();
    let gw = glyph.width as usize;
    let gh = glyph.height as usize;

    for gy in 0..gh {
        let screen_y = py as i32 + glyph.bearing_y as i32 + gy as i32;
        if screen_y < 0 || screen_y >= sh as i32 { continue; }
        let sy = screen_y as usize;

        for gx in 0..gw {
            let screen_x = px as i32 + glyph.bearing_x as i32 + gx as i32;
            if screen_x < 0 || screen_x >= sw as i32 { continue; }
            let sx = screen_x as usize;

            let coverage = glyph.data[gy * gw + gx];
            if coverage == 0 { continue; }

            let (bg_r, bg_g, bg_b) = fb.get_pixel(sx, sy);
            let (r, g, b) = alpha_blend(color, bg_r, bg_g, bg_b, coverage);
            fb.set_pixel(sx, sy, r, g, b);
        }
    }
}

/// Draw an anti-aliased text string using the given font face.
/// Returns the total width rendered.
pub fn draw_text_with_face(fb: &mut FramebufferManager, text: &str, px: usize, py: usize,
                           color: Color, face: &FontFace) -> usize {
    let mut cursor_x = px;
    for byte in text.bytes() {
        if byte == b'\n' { break; }
        let idx = (byte as usize).wrapping_sub(32);
        let glyph = if idx < face.glyphs.len() {
            &face.glyphs[idx]
        } else {
            &face.glyphs[0] // space fallback
        };

        draw_glyph_aa(fb, glyph, cursor_x, py, color);
        cursor_x += glyph.advance as usize;
    }
    cursor_x - px
}

/// Measure the width of a text string in pixels using the given font face.
pub fn measure_text_with_face(text: &str, face: &FontFace) -> usize {
    let mut width = 0usize;
    for byte in text.bytes() {
        if byte == b'\n' { break; }
        let idx = (byte as usize).wrapping_sub(32);
        let glyph = if idx < face.glyphs.len() {
            &face.glyphs[idx]
        } else {
            &face.glyphs[0]
        };
        width += glyph.advance as usize;
    }
    width
}
