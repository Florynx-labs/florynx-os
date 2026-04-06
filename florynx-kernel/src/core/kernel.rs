// =============================================================================
// Florynx Kernel — Kernel Core Logic
// =============================================================================
// The central kernel initialization and orchestration module.
// =============================================================================

/// Display a kernel banner on the VGA screen.
pub fn print_banner() {
    use crate::drivers::display::vga::{Color, WRITER};
    use core::fmt::Write;

    let mut writer = WRITER.lock();

    // Title in bright color
    writer.set_color(Color::LightCyan, Color::Black);
    let _ = write!(writer, "=== ");
    writer.set_color(Color::Yellow, Color::Black);
    let _ = write!(writer, "Florynx Kernel v0.1");
    writer.set_color(Color::LightCyan, Color::Black);
    let _ = writeln!(writer, " ===");

    // Boot message
    writer.set_color(Color::LightGreen, Color::Black);
    let _ = writeln!(writer, "Florynx Kernel v0.1 boot successful");

    // Reset to default
    writer.set_color(Color::LightGreen, Color::Black);
    let _ = writeln!(writer, "");
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
