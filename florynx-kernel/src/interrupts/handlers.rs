// =============================================================================
// Florynx Kernel — Common Interrupt Handlers
// =============================================================================
// Shared interrupt handling utilities used across the interrupt subsystem.
// =============================================================================

/// Log an unhandled interrupt to serial.
pub fn unhandled_interrupt(vector: u8) {
    crate::serial_println!("[interrupt] unhandled vector: {}", vector);
}

/// Disable all maskable interrupts.
pub fn disable() {
    x86_64::instructions::interrupts::disable();
}

/// Enable all maskable interrupts.
pub fn enable() {
    x86_64::instructions::interrupts::enable();
}

/// Execute a closure with interrupts disabled.
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    x86_64::instructions::interrupts::without_interrupts(f)
}
