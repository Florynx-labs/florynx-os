// =============================================================================
// Florynx Userland — Text Editor (Kate-Style)
// =============================================================================

/// Text editor state.
pub struct TextEditor {
    pub buffer: [u8; 8192],
    pub buf_len: usize,
    pub cursor_pos: usize,
    pub line_numbers: bool,
    pub word_wrap: bool,
}

impl TextEditor {
    pub fn new() -> Self {
        TextEditor {
            buffer: [0u8; 8192],
            buf_len: 0,
            cursor_pos: 0,
            line_numbers: true,
            word_wrap: true,
        }
    }
}
