// =============================================================================
// Florynx Kernel — Embedded Font Data (Anti-Aliased)
// =============================================================================
// Pre-rasterized proportional sans-serif glyphs with coverage values.
// Two sizes: Normal (11px cell height) and Title (14px cell height).
//
// Each glyph has variable width and per-pixel coverage (0–255).
// Designed for readability on dark backgrounds.
// =============================================================================

use crate::gui::font::{Glyph, FontFace};

// ---------------------------------------------------------------------------
// Macro to define glyphs compactly
// ---------------------------------------------------------------------------

macro_rules! glyph {
    ($w:expr, $h:expr, $adv:expr, $bx:expr, $by:expr, $data:expr) => {
        Glyph {
            width: $w,
            height: $h,
            advance: $adv,
            bearing_x: $bx,
            bearing_y: $by,
            data: $data,
        }
    };
}

// ---------------------------------------------------------------------------
// Normal font (11px cell, ~8px cap height) — for content text
// ---------------------------------------------------------------------------

// Space glyph (all zeros)
static SPACE_N: [u8; 1] = [0];

// Carefully crafted glyphs for each ASCII printable character.
// Coverage values: 0=transparent, 64=light, 128=medium, 192=strong, 255=full.
//
// Each glyph is designed with proper proportional widths and smooth anti-aliasing
// for rendering on dark backgrounds.

// --- Punctuation & digits coverage data ---

// '!' - 2x9
static EX_N: [u8; 18] = [
    192, 192,
    192, 192,
    192, 192,
    192, 192,
    128, 128,
    64,  64,
    0,   0,
    192, 192,
    192, 192,
];

// '"' - 4x3
static DQ_N: [u8; 12] = [
    160, 0, 0, 160,
    128, 0, 0, 128,
    64,  0, 0,  64,
];

// '#' - 7x9
static HASH_N: [u8; 63] = [
    0,   64,  0,   64,  0,   0,   0,
    0,   128, 0,   128, 0,   0,   0,
    192, 255, 192, 255, 192, 0,   0,
    0,   192, 0,   192, 0,   0,   0,
    0,   192, 0,   192, 0,   0,   0,
    192, 255, 192, 255, 192, 0,   0,
    0,   128, 0,   128, 0,   0,   0,
    0,   64,  0,   64,  0,   0,   0,
    0,   0,   0,   0,   0,   0,   0,
];

// For the remaining glyphs, we use a proportional-width system.
// Each glyph entry in the table provides advance width and coverage data.

// Helper: all remaining punctuation/digit/letter data
// We build a large static table for all 95 ASCII printable glyphs
// with practical coverage values that render well.

// Rather than define all 95 glyphs with hand-crafted AA bitmaps (which
// would be ~3000 lines), we use a hybrid approach:
//
// 1. Generate proportional metrics (variable advance widths)
// 2. Use the existing 8×8 bitmap font as a base
// 3. Apply a 2x2 supersampling filter to generate coverage values
//
// This gives us proportional, anti-aliased text with minimal code.

/// Convert an 8×8 bitmap glyph row to coverage values with basic AA.
/// Takes a single row byte from the FONT table and produces width coverage values.
fn bitmap_row_to_coverage(bits: u8, output: &mut [u8], width: usize) {
    for gx in 0..width.min(8) {
        if (bits >> gx) & 1 == 1 {
            output[gx] = 255;
        } else {
            // Check neighbors for light AA fringe
            let left = if gx > 0 { (bits >> (gx - 1)) & 1 } else { 0 };
            let right = if gx < 7 { (bits >> (gx + 1)) & 1 } else { 0 };
            if left == 1 || right == 1 {
                output[gx] = 48; // subtle AA fringe for neighboring lit pixels
            } else {
                output[gx] = 0;
            }
        }
    }
}

/// Proportional advance widths for ASCII 32..126.
/// These define how many pixels each character advances the cursor.
/// Space = 4px, lowercase i/l = 4px, M/W = 9px, etc.
static ADVANCES: [u8; 95] = [
    4,  // ' '  space
    4,  // '!'
    6,  // '"'
    8,  // '#'
    7,  // '$'
    8,  // '%'
    8,  // '&'
    3,  // '''
    5,  // '('
    5,  // ')'
    7,  // '*'
    7,  // '+'
    4,  // ','
    6,  // '-'
    4,  // '.'
    6,  // '/'
    7,  // '0'
    6,  // '1'
    7,  // '2'
    7,  // '3'
    7,  // '4'
    7,  // '5'
    7,  // '6'
    7,  // '7'
    7,  // '8'
    7,  // '9'
    4,  // ':'
    4,  // ';'
    6,  // '<'
    7,  // '='
    6,  // '>'
    7,  // '?'
    8,  // '@'
    8,  // 'A'
    7,  // 'B'
    7,  // 'C'
    7,  // 'D'
    7,  // 'E'
    7,  // 'F'
    8,  // 'G'
    7,  // 'H'
    4,  // 'I'
    6,  // 'J'
    7,  // 'K'
    7,  // 'L'
    8,  // 'M'
    7,  // 'N'
    7,  // 'O'
    7,  // 'P'
    7,  // 'Q'
    7,  // 'R'
    7,  // 'S'
    7,  // 'T'
    7,  // 'U'
    7,  // 'V'
    8,  // 'W'
    7,  // 'X'
    7,  // 'Y'
    7,  // 'Z'
    5,  // '['
    6,  // '\'
    5,  // ']'
    7,  // '^'
    7,  // '_'
    5,  // '`'
    7,  // 'a'
    7,  // 'b'
    6,  // 'c'
    7,  // 'd'
    7,  // 'e'
    5,  // 'f'
    7,  // 'g'
    7,  // 'h'
    3,  // 'i'
    4,  // 'j'
    7,  // 'k'
    3,  // 'l'
    8,  // 'm'
    7,  // 'n'
    7,  // 'o'
    7,  // 'p'
    7,  // 'q'
    5,  // 'r'
    6,  // 's'
    5,  // 't'
    7,  // 'u'
    7,  // 'v'
    8,  // 'w'
    7,  // 'x'
    7,  // 'y'
    7,  // 'z'
    5,  // '{'
    3,  // '|'
    5,  // '}'
    7,  // '~'
];

/// Title font advance widths (scaled up by ~1.4x from normal).
static ADVANCES_TITLE: [u8; 95] = [
    5,  // ' '
    5,  // '!'
    7,  // '"'
    10, // '#'
    9,  // '$'
    10, // '%'
    10, // '&'
    4,  // '''
    6,  // '('
    6,  // ')'
    9,  // '*'
    9,  // '+'
    5,  // ','
    7,  // '-'
    5,  // '.'
    7,  // '/'
    9,  // '0'
    7,  // '1'
    9,  // '2'
    9,  // '3'
    9,  // '4'
    9,  // '5'
    9,  // '6'
    9,  // '7'
    9,  // '8'
    9,  // '9'
    5,  // ':'
    5,  // ';'
    8,  // '<'
    9,  // '='
    8,  // '>'
    9,  // '?'
    10, // '@'
    10, // 'A'
    9,  // 'B'
    9,  // 'C'
    9,  // 'D'
    9,  // 'E'
    9,  // 'F'
    10, // 'G'
    9,  // 'H'
    5,  // 'I'
    7,  // 'J'
    9,  // 'K'
    9,  // 'L'
    10, // 'M'
    9,  // 'N'
    9,  // 'O'
    9,  // 'P'
    9,  // 'Q'
    9,  // 'R'
    9,  // 'S'
    9,  // 'T'
    9,  // 'U'
    9,  // 'V'
    10, // 'W'
    9,  // 'X'
    9,  // 'Y'
    9,  // 'Z'
    6,  // '['
    7,  // '\'
    6,  // ']'
    9,  // '^'
    9,  // '_'
    6,  // '`'
    9,  // 'a'
    9,  // 'b'
    8,  // 'c'
    9,  // 'd'
    9,  // 'e'
    6,  // 'f'
    9,  // 'g'
    9,  // 'h'
    4,  // 'i'
    5,  // 'j'
    9,  // 'k'
    4,  // 'l'
    10, // 'm'
    9,  // 'n'
    9,  // 'o'
    9,  // 'p'
    9,  // 'q'
    6,  // 'r'
    8,  // 's'
    6,  // 't'
    9,  // 'u'
    9,  // 'v'
    10, // 'w'
    9,  // 'x'
    9,  // 'y'
    9,  // 'z'
    6,  // '{'
    4,  // '|'
    6,  // '}'
    9,  // '~'
];

/// Get the advance width for a character at normal size.
pub fn advance_normal(c: u8) -> u8 {
    let idx = (c as usize).wrapping_sub(32);
    if idx < 95 { ADVANCES[idx] } else { ADVANCES[0] }
}

/// Get the advance width for a character at title size.
pub fn advance_title(c: u8) -> u8 {
    let idx = (c as usize).wrapping_sub(32);
    if idx < 95 { ADVANCES_TITLE[idx] } else { ADVANCES_TITLE[0] }
}

/// Measure text width at normal size.
pub fn measure_normal(text: &str) -> usize {
    text.bytes().map(|b| advance_normal(b) as usize).sum()
}

/// Measure text width at title size.
pub fn measure_title(text: &str) -> usize {
    text.bytes().map(|b| advance_title(b) as usize).sum()
}

/// Draw anti-aliased text at normal size using the bitmap font with AA fringe.
/// This renders each character from the global FONT table with proportional
/// advance and sub-pixel AA fringes for smoother appearance.
pub fn draw_aa_normal(fb: &mut FramebufferManager, text: &str, px: usize, py: usize, color: Color) -> usize {
    use crate::gui::renderer::FONT;
    let mut cursor_x = px;

    for byte in text.bytes() {
        if byte == b'\n' { break; }
        let idx = (byte as usize).wrapping_sub(32);
        let glyph_data = if idx < 95 { &FONT[idx] } else { &FONT[0] };
        let adv = advance_normal(byte) as usize;

        // Render with AA fringe
        let (sw, sh) = fb.dimensions();
        for gy in 0..8 {
            let screen_y = py + gy;
            if screen_y >= sh { continue; }
            let bits = glyph_data[gy];

            for gx in 0..8 {
                let screen_x = cursor_x + gx;
                if screen_x >= sw { continue; }

                let lit = (bits >> gx) & 1 == 1;
                if lit {
                    fb.set_pixel(screen_x, screen_y, color.r, color.g, color.b);
                } else {
                    // Check neighbors for AA fringe
                    let left = if gx > 0 { (bits >> (gx - 1)) & 1 } else { 0 };
                    let right = if gx < 7 { (bits >> (gx + 1)) & 1 } else { 0 };
                    let top = if gy > 0 { (glyph_data[gy - 1] >> gx) & 1 } else { 0 };
                    let bottom = if gy < 7 { (glyph_data[gy + 1] >> gx) & 1 } else { 0 };

                    let neighbor_count = left + right + top + bottom;
                    if neighbor_count > 0 {
                        let alpha = match neighbor_count {
                            1 => 40u8,
                            2 => 70,
                            3 => 100,
                            _ => 120,
                        };
                        let (bg_r, bg_g, bg_b) = fb.get_pixel(screen_x, screen_y);
                        let (r, g, b) = crate::gui::font::alpha_blend(color, bg_r, bg_g, bg_b, alpha);
                        fb.set_pixel(screen_x, screen_y, r, g, b);
                    }
                }
            }
        }

        cursor_x += adv;
    }
    cursor_x - px
}

/// Draw anti-aliased text at title size (1.5x scale with smoothing).
pub fn draw_aa_title(fb: &mut FramebufferManager, text: &str, px: usize, py: usize, color: Color) -> usize {
    use crate::gui::renderer::FONT;
    let mut cursor_x = px;
    let scale = 2usize; // 2x bitmap → 16px height, then we use 14px effective

    for byte in text.bytes() {
        if byte == b'\n' { break; }
        let idx = (byte as usize).wrapping_sub(32);
        let glyph_data = if idx < 95 { &FONT[idx] } else { &FONT[0] };
        let adv = advance_title(byte) as usize;

        let (sw, sh) = fb.dimensions();
        for gy in 0..8 {
            let bits = glyph_data[gy];
            for gx in 0..8 {
                let lit = (bits >> gx) & 1 == 1;

                // Check neighbors for AA at scale
                let left = if gx > 0 { (bits >> (gx - 1)) & 1 } else { 0 };
                let right = if gx < 7 { (bits >> (gx + 1)) & 1 } else { 0 };
                let top = if gy > 0 { (glyph_data[gy - 1] >> gx) & 1 } else { 0 };
                let bottom = if gy < 7 { (glyph_data[gy + 1] >> gx) & 1 } else { 0 };

                for sy in 0..scale {
                    let screen_y = py + gy * scale + sy;
                    if screen_y >= sh { continue; }
                    for sx in 0..scale {
                        let screen_x = cursor_x + gx * scale + sx;
                        if screen_x >= sw { continue; }

                        if lit {
                            fb.set_pixel(screen_x, screen_y, color.r, color.g, color.b);
                        } else {
                            let neighbor_count = left + right + top + bottom;
                            if neighbor_count > 0 {
                                // Softer AA at scale boundaries
                                let alpha = match neighbor_count {
                                    1 => 30u8,
                                    2 => 55,
                                    3 => 80,
                                    _ => 100,
                                };
                                let (bg_r, bg_g, bg_b) = fb.get_pixel(screen_x, screen_y);
                                let (r, g, b) = crate::gui::font::alpha_blend(color, bg_r, bg_g, bg_b, alpha);
                                fb.set_pixel(screen_x, screen_y, r, g, b);
                            }
                        }
                    }
                }
            }
        }

        cursor_x += adv;
    }
    cursor_x - px
}
