// =============================================================================
// Florynx Kernel — GUI Menu Bar Component
// =============================================================================
// macOS-style global menu bar at the top of the screen.
// Left: Florynx logo glyph + active window title.
// Right: uptime clock (HH:MM:SS).
// =============================================================================

use crate::gui::renderer::{self, FramebufferManager};
use crate::gui::theme;
use crate::gui::event::Rect;

const MAX_TITLE: usize = 48;

pub struct MenuBar {
    title: [u8; MAX_TITLE],
    title_len: usize,
    screen_w: usize,
}

impl MenuBar {
    pub const fn new(screen_w: usize) -> Self {
        MenuBar {
            title: [0u8; MAX_TITLE],
            title_len: 0,
            screen_w,
        }
    }

    pub fn set_title(&mut self, text: &str) {
        let len = text.len().min(MAX_TITLE);
        self.title[..len].copy_from_slice(&text.as_bytes()[..len]);
        self.title_len = len;
    }

    fn title_str(&self) -> &str {
        core::str::from_utf8(&self.title[..self.title_len]).unwrap_or("")
    }

    pub fn rect(&self) -> Rect {
        Rect::new(0, 0, self.screen_w, theme::DARK.menubar_h)
    }

    pub fn draw(&self, fb: &mut FramebufferManager) {
        let t = &theme::DARK;
        let h = t.menubar_h;

        // Semi-transparent dark background
        renderer::draw_rect(fb, 0, 0, self.screen_w, h, t.menubar_bg);

        // Bottom separator line
        renderer::draw_hline(fb, 0, h - 1, self.screen_w, t.border);

        // Left side: Florynx logo glyph + bold app title
        renderer::draw_text(fb, "F", 12, (h.saturating_sub(8)) / 2, t.accent, 1);

        // Active window title (bold = scale 1, white)
        let title = self.title_str();
        if !title.is_empty() {
            renderer::draw_text(fb, title, 28, (h.saturating_sub(8)) / 2, t.text, 1);
        }

        // Right side: real wall-clock time from RTC
        let rtc = crate::time::rtc::now_rtc();

        // Format HH:MM:SS into a fixed buffer
        let mut clock_buf = [0u8; 8];
        clock_buf[0] = b'0' + (rtc.hours / 10);
        clock_buf[1] = b'0' + (rtc.hours % 10);
        clock_buf[2] = b':';
        clock_buf[3] = b'0' + (rtc.minutes / 10);
        clock_buf[4] = b'0' + (rtc.minutes % 10);
        clock_buf[5] = b':';
        clock_buf[6] = b'0' + (rtc.seconds / 10);
        clock_buf[7] = b'0' + (rtc.seconds % 10);
        let clock_str = core::str::from_utf8(&clock_buf).unwrap_or("00:00:00");

        let clock_w = 8 * 8; // 8 chars × 8px
        let clock_x = self.screen_w.saturating_sub(clock_w + 14);
        renderer::draw_text(fb, clock_str, clock_x, (h.saturating_sub(8)) / 2, t.text_dim, 1);
    }
}
