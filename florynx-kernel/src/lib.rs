// =============================================================================
// Florynx Kernel — Library Root (lib.rs)
// =============================================================================
// Central module declarations and initialization entrypoint.
// This is the library crate root, used by main.rs for the kernel binary.
// =============================================================================

#![no_std]
#![feature(abi_x86_interrupt)]

extern crate alloc;

// ---------------------------------------------------------------------------
// Module declarations
// ---------------------------------------------------------------------------
pub mod arch;
#[path = "core/mod.rs"]
pub mod core_kernel;
pub mod drivers;
pub mod memory;
pub mod process;
pub mod syscall;
pub mod interrupts;
pub mod ipc;
pub mod security;
pub mod fs;
pub mod time;
pub mod gui;
pub mod runtime;
pub mod wincompat;

// Re-export commonly used items
pub use core_kernel::logging::LogLevel;

/// Initialize core kernel subsystems (CPU tables + interrupt controllers).
/// Called early in the boot process after the bootloader hands off control.
/// NOTE: Does NOT enable interrupts. The caller must do that after heap init.
pub fn init(_boot_info: &'static bootloader::BootInfo) {
    // 1. GDT (must be first — sets up segment selectors and TSS)
    arch::x86_64::gdt::init();

    // 2. IDT (registers exception and IRQ handlers)
    arch::x86_64::idt::init();

    // 3. Hardware interrupts (PIC + PIT configured, but NOT enabled yet)
    arch::x86_64::interrupts::init();

    // 4. Driver registry setup (persistent instances, no per-IRQ construction)
    drivers::init_registry();

    serial_println!("[kernel] core init complete (interrupts still disabled)");
}

/// Initialize display subsystem (BGA framebuffer + text console + mouse).
/// Must be called AFTER heap is initialized and BEFORE interrupts are enabled.
/// Mouse init MUST happen here (before interrupts) to avoid IRQ12 race.
pub fn init_gui(boot_info: &'static bootloader::BootInfo) {
    // BGA framebuffer
    drivers::display::bga::init(boot_info.physical_memory_offset);

    // Initialize framebuffer text console (clears screen to black, used for early boot)
    gui::console::init();

    // Initialize PS/2 mouse BEFORE interrupts are enabled (avoids IRQ12 race)
    if !drivers::input::mouse::init() {
        serial_println!("[kernel] WARNING: mouse init failed, continuing without mouse");
    }

    serial_println!("[kernel] display subsystem initialized");
}

/// Launch the graphical desktop. Call AFTER interrupts are enabled.
pub fn launch_desktop() {
    // Initialize and draw the desktop GUI
    gui::desktop::init();
    gui::desktop::draw();

    // Draw initial cursor at center
    gui::renderer::update_cursor(400, 300);

    serial_println!("[kernel] desktop GUI launched");
}

/// Launch HGUI mode: compositor core only, no kernel demo windows.
/// Userland is expected to own the visible shell through GUI syscalls.
pub fn launch_hgui() {
    gui::hgui_link::launch_core();
    serial_println!("[kernel] HGUI mode launched");
}

/// Halt loop — stops the CPU until the next interrupt.
pub fn hlt_loop() -> ! {
    serial_println!("[kernel] entering stable hlt_loop");
    loop {
        x86_64::instructions::hlt();
    }
}
