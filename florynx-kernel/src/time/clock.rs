// =============================================================================
// Florynx Kernel — System Clock
// =============================================================================
// Provides wall-clock time and uptime tracking backed by the PIT timer.
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

/// Boot timestamp (seconds since Unix epoch, set during init).
static BOOT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

/// Initialize the clock subsystem (reads RTC for real wall-clock time).
pub fn init() {
    crate::time::rtc::init();
    BOOT_TIMESTAMP.store(crate::time::rtc::boot_epoch(), Ordering::Relaxed);
    crate::serial_println!("[clock] initialized (wall-clock from RTC)");
}

/// Get the current Unix timestamp (wall-clock seconds since 1970-01-01).
pub fn now_unix() -> u64 {
    crate::time::rtc::now_unix()
}

/// Get the system uptime in seconds.
pub fn uptime() -> u64 {
    crate::drivers::timer::pit::uptime_seconds()
}

/// Get the system uptime in ticks.
pub fn uptime_ticks() -> u64 {
    crate::drivers::timer::pit::get_ticks()
}

/// Busy-wait for the given number of ticks.
pub fn wait_ticks(ticks: u64) {
    let start = uptime_ticks();
    while uptime_ticks() - start < ticks {
        x86_64::instructions::hlt();
    }
}
