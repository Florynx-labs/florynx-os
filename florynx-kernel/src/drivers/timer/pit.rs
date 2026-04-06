// =============================================================================
// Florynx Kernel — PIT (Programmable Interval Timer) Driver
// =============================================================================
// Configures the 8254 PIT for periodic timer interrupts.
// Channel 0 is used for the system timer at ~100 Hz.
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::instructions::port::Port;

/// System tick counter, incremented by each timer interrupt.
static TICKS: AtomicU64 = AtomicU64::new(0);

/// PIT oscillator base frequency (~1.193182 MHz).
const PIT_FREQUENCY: u32 = 1_193_182;

/// Desired timer frequency in Hz (200 Hz = 5ms per tick for smooth GUI).
const TARGET_FREQUENCY: u32 = 200;

/// PIT I/O ports.
const PIT_CHANNEL0: u16 = 0x40;
const PIT_COMMAND: u16 = 0x43;

/// Initialize the PIT to fire at TARGET_FREQUENCY Hz.
pub fn init() {
    let divisor = PIT_FREQUENCY / TARGET_FREQUENCY;
    let low = (divisor & 0xFF) as u8;
    let high = ((divisor >> 8) & 0xFF) as u8;

    unsafe {
        // Channel 0, lobyte/hibyte access, rate generator mode
        let mut cmd_port = Port::new(PIT_COMMAND);
        cmd_port.write(0x36u8);

        let mut data_port = Port::new(PIT_CHANNEL0);
        data_port.write(low);
        data_port.write(high);
    }

    crate::serial_println!("[pit] initialized at {} Hz", TARGET_FREQUENCY);
}

/// Called by the timer interrupt handler. Increments the tick counter.
pub fn tick() {
    TICKS.fetch_add(1, Ordering::Relaxed);
}

/// Returns the number of ticks since boot.
pub fn get_ticks() -> u64 {
    TICKS.load(Ordering::Relaxed)
}

/// Returns approximate uptime in seconds.
pub fn uptime_seconds() -> u64 {
    get_ticks() / TARGET_FREQUENCY as u64
}
