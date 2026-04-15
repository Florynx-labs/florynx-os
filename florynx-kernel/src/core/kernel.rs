// =============================================================================
// Florynx Kernel — Kernel Core Logic
// =============================================================================
// The central kernel initialization and orchestration module.
// =============================================================================

/// Display a kernel banner on the console.
pub fn print_banner() {
    crate::println!("=== Florynx Kernel v0.4.5 ===");
    crate::println!("Florynx Kernel boot successful\n");
}

/// Called after all subsystems are initialized.
pub fn post_init() {
    crate::serial_println!("[kernel] all subsystems initialized");
    crate::println!("[kernel] all subsystems initialized");

    // Log CPU info
    crate::arch::x86_64::cpu::log_cpu_info();

    // Initialize clock
    crate::time::clock::init();
}
