// =============================================================================
// Florynx Kernel — Panic Handler
// =============================================================================
// Handles kernel panics by printing diagnostic info to both VGA and serial.
// =============================================================================

use core::panic::PanicInfo;

/// The kernel panic handler — prints error info and halts.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Print to serial (always available)
    crate::serial_println!("!!! KERNEL PANIC !!!");
    crate::serial_println!("{}", info);

    // Also try to print to VGA 
    crate::println!("!!! KERNEL PANIC !!!");
    crate::println!("{}", info);

    // Halt the CPU in an infinite loop
    loop {
        x86_64::instructions::hlt();
    }
}
