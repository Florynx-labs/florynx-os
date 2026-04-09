// =============================================================================
// Florynx Userland — System Tray (KDE Plasma-Style)
// =============================================================================
// Right side of the panel: clock, volume, network, battery indicators.
// =============================================================================

/// System tray widget state.
pub struct SystemTray {
    pub screen_w: usize,
    pub tray_x: usize,
    pub tray_w: usize,
    pub clock_text: [u8; 8],  // "HH:MM:SS"
    pub clock_len: usize,
}

impl SystemTray {
    pub fn new(tray_x: usize, tray_w: usize, screen_w: usize) -> Self {
        SystemTray {
            screen_w,
            tray_x,
            tray_w,
            clock_text: *b"00:00:00",
            clock_len: 8,
        }
    }

    /// Update clock display from system time (hours, minutes, seconds).
    pub fn update_clock(&mut self, h: u8, m: u8, s: u8) {
        self.clock_text[0] = b'0' + (h / 10);
        self.clock_text[1] = b'0' + (h % 10);
        self.clock_text[2] = b':';
        self.clock_text[3] = b'0' + (m / 10);
        self.clock_text[4] = b'0' + (m % 10);
        self.clock_text[5] = b':';
        self.clock_text[6] = b'0' + (s / 10);
        self.clock_text[7] = b'0' + (s % 10);
    }

    pub fn clock_str(&self) -> &str {
        core::str::from_utf8(&self.clock_text[..self.clock_len]).unwrap_or("??:??:??")
    }
}
