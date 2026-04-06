// =============================================================================
// Florynx Kernel — Icon System
// =============================================================================
// Embedded bitmap icons for dock items and window controls.
// Supports 8x8, 16x16, and 32x32 monochrome bitmaps.
// =============================================================================

use crate::gui::renderer::{Color, FramebufferManager};

// ---------------------------------------------------------------------------
// Icon structure
// ---------------------------------------------------------------------------

pub struct Icon {
    pub width: usize,
    pub height: usize,
    pub data: &'static [u8],
}

impl Icon {
    pub const fn new(width: usize, height: usize, data: &'static [u8]) -> Self {
        Icon { width, height, data }
    }
}

// ---------------------------------------------------------------------------
// Draw icon with color
// ---------------------------------------------------------------------------

pub fn draw_icon(fb: &mut FramebufferManager, icon: &Icon, x: usize, y: usize, color: Color) {
    let bytes_per_row = (icon.width + 7) / 8;
    for py in 0..icon.height {
        for px in 0..icon.width {
            let byte_idx = py * bytes_per_row + px / 8;
            let bit_idx = 7 - (px % 8);
            if byte_idx < icon.data.len() {
                if (icon.data[byte_idx] >> bit_idx) & 1 != 0 {
                    fb.set_pixel(x + px, y + py, color.r, color.g, color.b);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 16x16 Icons for dock items
// ---------------------------------------------------------------------------

// Folder icon (16x16)
#[rustfmt::skip]
const ICON_FOLDER_DATA: [u8; 32] = [
    0b00000000, 0b00000000,
    0b01111000, 0b00000000,
    0b01001111, 0b11111000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001000, 0b00001000,
    0b01001111, 0b11111000,
    0b01111111, 0b11111110,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

pub static ICON_FOLDER: Icon = Icon::new(16, 16, &ICON_FOLDER_DATA);

// Terminal icon (16x16)
#[rustfmt::skip]
const ICON_TERMINAL_DATA: [u8; 32] = [
    0b11111111, 0b11111111,
    0b10000000, 0b00000001,
    0b10000000, 0b00000001,
    0b10110000, 0b00000001,
    0b10011000, 0b00000001,
    0b10001100, 0b00000001,
    0b10000110, 0b00000001,
    0b10001100, 0b00000001,
    0b10011000, 0b00000001,
    0b10110000, 0b00000001,
    0b10000000, 0b00000001,
    0b10000000, 0b00111001,
    0b10000000, 0b00000001,
    0b10000000, 0b00000001,
    0b11111111, 0b11111111,
    0b00000000, 0b00000000,
];

pub static ICON_TERMINAL: Icon = Icon::new(16, 16, &ICON_TERMINAL_DATA);

// Settings/gear icon (16x16)
#[rustfmt::skip]
const ICON_SETTINGS_DATA: [u8; 32] = [
    0b00000000, 0b00000000,
    0b00000110, 0b01100000,
    0b00001111, 0b11110000,
    0b00011111, 0b11111000,
    0b00111110, 0b01111100,
    0b01111100, 0b00111110,
    0b01111000, 0b00011110,
    0b01110000, 0b00001110,
    0b01110000, 0b00001110,
    0b01111000, 0b00011110,
    0b01111100, 0b00111110,
    0b00111110, 0b01111100,
    0b00011111, 0b11111000,
    0b00001111, 0b11110000,
    0b00000110, 0b01100000,
    0b00000000, 0b00000000,
];

pub static ICON_SETTINGS: Icon = Icon::new(16, 16, &ICON_SETTINGS_DATA);

// Monitor/display icon (16x16)
#[rustfmt::skip]
const ICON_MONITOR_DATA: [u8; 32] = [
    0b00000000, 0b00000000,
    0b01111111, 0b11111110,
    0b01000000, 0b00000010,
    0b01011111, 0b11111010,
    0b01010000, 0b00001010,
    0b01010000, 0b00001010,
    0b01010000, 0b00001010,
    0b01010000, 0b00001010,
    0b01010000, 0b00001010,
    0b01011111, 0b11111010,
    0b01000000, 0b00000010,
    0b01111111, 0b11111110,
    0b00000011, 0b11000000,
    0b00001111, 0b11110000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

pub static ICON_MONITOR: Icon = Icon::new(16, 16, &ICON_MONITOR_DATA);

// Document/notes icon (16x16)
#[rustfmt::skip]
const ICON_DOCUMENT_DATA: [u8; 32] = [
    0b00000000, 0b00000000,
    0b00111111, 0b11000000,
    0b00100000, 0b01100000,
    0b00100000, 0b00110000,
    0b00100000, 0b00011000,
    0b00100000, 0b00001000,
    0b00111111, 0b11111000,
    0b00100000, 0b00001000,
    0b00111100, 0b00001000,
    0b00100000, 0b00001000,
    0b00111100, 0b00001000,
    0b00100000, 0b00001000,
    0b00100000, 0b00001000,
    0b00111111, 0b11111000,
    0b00000000, 0b00000000,
    0b00000000, 0b00000000,
];

pub static ICON_DOCUMENT: Icon = Icon::new(16, 16, &ICON_DOCUMENT_DATA);

// ---------------------------------------------------------------------------
// 8x8 Icons for window controls
// ---------------------------------------------------------------------------

// Close X icon (8x8)
#[rustfmt::skip]
const ICON_CLOSE_DATA: [u8; 8] = [
    0b11000011,
    0b01100110,
    0b00111100,
    0b00011000,
    0b00011000,
    0b00111100,
    0b01100110,
    0b11000011,
];

pub static ICON_CLOSE: Icon = Icon::new(8, 8, &ICON_CLOSE_DATA);

// Minimize - icon (8x8)
#[rustfmt::skip]
const ICON_MINIMIZE_DATA: [u8; 8] = [
    0b00000000,
    0b00000000,
    0b00000000,
    0b00000000,
    0b00000000,
    0b00000000,
    0b11111111,
    0b00000000,
];

pub static ICON_MINIMIZE: Icon = Icon::new(8, 8, &ICON_MINIMIZE_DATA);

// Maximize square icon (8x8)
#[rustfmt::skip]
const ICON_MAXIMIZE_DATA: [u8; 8] = [
    0b11111111,
    0b10000001,
    0b10000001,
    0b10000001,
    0b10000001,
    0b10000001,
    0b10000001,
    0b11111111,
];

pub static ICON_MAXIMIZE: Icon = Icon::new(8, 8, &ICON_MAXIMIZE_DATA);
