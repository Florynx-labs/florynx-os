// =============================================================================
// Florynx Kernel — Arch-level Interrupt Setup
// =============================================================================
// Hardware interrupt initialization for x86_64: PIC setup and interrupt enable.
// =============================================================================

/// Initialize hardware interrupts: PIC setup and PIT timer.
/// NOTE: Does NOT enable interrupts — the caller must do that
/// only after all subsystems (heap, GUI, etc.) are fully ready.
pub fn init() {
    // Initialize the chained PICs
    crate::interrupts::pic::init();

    // Initialize the PIT timer
    crate::drivers::timer::pit::init();

    // Do NOT enable interrupts here. The main init sequence controls that.
    crate::serial_println!("[interrupts] PIC + PIT initialized (interrupts still disabled)");
}
