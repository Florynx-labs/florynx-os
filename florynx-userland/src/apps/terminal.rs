// =============================================================================
// Florynx Userland — Terminal (Konsole-Style)
// =============================================================================

/// Terminal emulator state.
pub struct Terminal {
    pub buffer: [u8; 4096],
    pub buf_len: usize,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cols: usize,
    pub rows: usize,
    pub scroll_offset: usize,
}

impl Terminal {
    pub fn new(cols: usize, rows: usize) -> Self {
        Terminal {
            buffer: [0u8; 4096],
            buf_len: 0,
            cursor_row: 0,
            cursor_col: 0,
            cols,
            rows,
            scroll_offset: 0,
        }
    }

    pub fn write_byte(&mut self, b: u8) {
        if self.buf_len < self.buffer.len() {
            self.buffer[self.buf_len] = b;
            self.buf_len += 1;
        }
    }

    pub fn write_str(&mut self, s: &str) {
        for b in s.bytes() {
            self.write_byte(b);
        }
    }
}
