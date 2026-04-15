// =============================================================================
// Florynx Kernel — GUI Menu Bar Component
// =============================================================================
// Design-aligned top bar:
//   Left: Rounded search bar (pill shape, "Rechercher" placeholder)
//   Right: Clock (HH:MM) + Date (DD/MM/YYYY)
// =============================================================================

use crate::gui::renderer::{self, FramebufferManager, Color, FontSize};
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

        // === LEFT: Search bar (pill shape) ===
        let search_x = 12;
        let search_y = 4;
        let search_w = 280;
        let search_h = h - 8;
        let search_r = search_h / 2; // fully rounded corners → pill shape

        // Search bar background (lighter for contrast)
        renderer::draw_rounded_rect(fb, search_x, search_y, search_w, search_h, search_r,
            Color::rgba(80, 85, 95, 160));
        renderer::draw_rounded_border(fb, search_x, search_y, search_w, search_h, search_r,
            Color::rgba(100, 105, 115, 100));

        // Search icon (magnifying glass - simple circle + line)
        let icon_x = search_x + 12;
        let icon_y = search_y + search_h / 2;
        renderer::draw_circle(fb, icon_x, icon_y, 4, Color::rgb(160, 168, 178));
        // Small line for handle
        renderer::draw_hline(fb, icon_x + 3, icon_y + 3, 3, Color::rgb(160, 168, 178));

        // Placeholder text "Rechercher" (using AA text)
        let placeholder = "Rechercher";
        let text_x = search_x + 28;
        let text_y = search_y + (search_h.saturating_sub(8)) / 2;
        renderer::draw_text_aa(fb, placeholder, text_x, text_y,
            Color::rgb(130, 138, 150), FontSize::Normal);

        // === RIGHT: Clock + Date ===
        let rtc = crate::time::rtc::now_rtc();

        // Format HH:MM
        let mut clock_buf = [0u8; 5];
        clock_buf[0] = b'0' + (rtc.hours / 10);
        clock_buf[1] = b'0' + (rtc.hours % 10);
        clock_buf[2] = b':';
        clock_buf[3] = b'0' + (rtc.minutes / 10);
        clock_buf[4] = b'0' + (rtc.minutes % 10);
        let clock_str = core::str::from_utf8(&clock_buf).unwrap_or("00:00");

        // Format DD/MM/YYYY
        let mut date_buf = [0u8; 10];
        date_buf[0] = b'0' + (rtc.day / 10);
        date_buf[1] = b'0' + (rtc.day % 10);
        date_buf[2] = b'/';
        date_buf[3] = b'0' + (rtc.month / 10);
        date_buf[4] = b'0' + (rtc.month % 10);
        date_buf[5] = b'/';
        // Year: RTC gives last 2 digits, assume 20xx
        let year = rtc.year;
        date_buf[6] = b'2';
        date_buf[7] = b'0';
        date_buf[8] = b'0' + ((year % 100) / 10) as u8;
        date_buf[9] = b'0' + (year % 10) as u8;
        let date_str = core::str::from_utf8(&date_buf).unwrap_or("01/01/2024");

        // Draw clock (normal weight, brighter)
        let clock_w = renderer::measure_text_aa(clock_str, FontSize::Normal);
        let date_w = renderer::measure_text_aa(date_str, FontSize::Normal);
        let gap = 14;
        let total_right = clock_w + gap + date_w;
        let right_x = self.screen_w.saturating_sub(total_right + 14);
        let text_y_center = (h.saturating_sub(8)) / 2;

        renderer::draw_text_aa(fb, clock_str, right_x, text_y_center, t.text, FontSize::Normal);
        renderer::draw_text_aa(fb, date_str, right_x + clock_w + gap, text_y_center,
            t.text_dim, FontSize::Normal);
    }
}
