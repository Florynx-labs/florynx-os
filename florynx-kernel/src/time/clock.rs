// =============================================================================
// Florynx Kernel — System Clock
// =============================================================================
// Provides wall-clock time and uptime tracking backed by the PIT timer.
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};

/// Boot timestamp (seconds since Unix epoch, set during init).
static BOOT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

/// Initialize the clock subsystem.
pub fn init() {
    // In a real kernel, we'd read the RTC to get wall-clock time.
    // For now, we just mark boot time as 0.
    BOOT_TIMESTAMP.store(0, Ordering::Relaxed);
    crate::serial_println!("[clock] initialized");
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
