// =============================================================================
// Florynx Kernel — GUI Menu Bar Component
// =============================================================================
// macOS-style global menu bar at the top of the screen.
// Left: Florynx logo glyph + active window title.
// Right: uptime clock (HH:MM:SS).
// =============================================================================

use crate::gui::renderer::{self, FramebufferManager, Color};
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

        // Fully transparent background (t.menubar_bg is transparent now)
        if t.menubar_bg.a > 0 {
            renderer::draw_rect(fb, 0, 0, self.screen_w, h, t.menubar_bg);
        }

        // Left side: Search Pill ("Rechercher")
        let search_w = 260;
        let search_h = 24;
        let search_y = (h.saturating_sub(search_h)) / 2;
        renderer::draw_rounded_rect(fb, 24, search_y, search_w, search_h, search_h / 2, Color::rgba(255, 255, 255, 80));
        renderer::draw_rounded_border(fb, 24, search_y, search_w, search_h, search_h / 2, Color::rgba(255, 255, 255, 120));
        renderer::draw_text(fb, "O Rechercher", 36, search_y + (search_h.saturating_sub(8)) / 2, t.text_dim, 1);

        // Right side: real wall-clock time from RTC + mockup icons
        let rtc = crate::time::rtc::now_rtc();

        // Format HH:MM into a fixed buffer
        let mut clock_buf = [0u8; 5];
        clock_buf[0] = b'0' + (rtc.hours / 10);
        clock_buf[1] = b'0' + (rtc.hours % 10);
        clock_buf[2] = b':';
        clock_buf[3] = b'0' + (rtc.minutes / 10);
        clock_buf[4] = b'0' + (rtc.minutes % 10);
        let clock_str = core::str::from_utf8(&clock_buf).unwrap_or("00:00");

        // Format Date DD/MM/YYYY
        let mut date_buf = [0u8; 10];
        date_buf[0] = b'0' + (rtc.day / 10);
        date_buf[1] = b'0' + (rtc.day % 10);
        date_buf[2] = b'/';
        date_buf[3] = b'0' + (rtc.month / 10);
        date_buf[4] = b'0' + (rtc.month % 10);
        date_buf[5] = b'/';
        let year = rtc.year as u16 + 2000;
        date_buf[6] = b'0' + (year / 1000) as u8;
        date_buf[7] = b'0' + ((year / 100) % 10) as u8;
        date_buf[8] = b'0' + ((year / 10) % 10) as u8;
        date_buf[9] = b'0' + (year % 10) as u8;
        let date_str = core::str::from_utf8(&date_buf).unwrap_or("01/01/2000");

        let status_str = "WIFI   BAT   ";
        let text_y = (h.saturating_sub(8)) / 2;
        
        let date_w = 10 * 8;
        let clock_w = 5 * 8;
        let status_w = status_str.len() * 8;
        
        let date_x = self.screen_w.saturating_sub(date_w + 24);
        let clock_x = date_x.saturating_sub(clock_w + 16);
        let status_x = clock_x.saturating_sub(status_w);

        renderer::draw_text(fb, status_str, status_x, text_y, t.text, 1);
        renderer::draw_text(fb, clock_str, clock_x, text_y, t.text, 1);
        renderer::draw_text(fb, date_str, date_x, text_y, t.text, 1);
    }
}
