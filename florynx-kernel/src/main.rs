// =============================================================================
// Florynx Kernel — Entry Point (main.rs) by asuno
// =============================================================================
// The kernel binary entry point. Uses the bootloader crate to receive
// boot information (memory map, framebuffer, etc.) and orchestrates
// the full kernel startup sequence.
//
// BOOT ORDER (stability-critical):
//   Phase 1: Core init (GDT, IDT, PIC+PIT) — interrupts DISABLED
//   Phase 2: Memory init (paging, frame allocator, heap)
//   Phase 3: Display init (BGA framebuffer, text console)
//   Phase 4: Enable interrupts — everything is now ready
//   Phase 5: Post-init (banner, CPU info)
//   Phase 6: Stable hlt_loop — ready for userland
// =============================================================================

#![no_std]
#![no_main]

extern crate alloc;

use bootloader::{entry_point, BootInfo};
use florynx_kernel::serial_println;
use x86_64::VirtAddr;

// Tell the bootloader crate where our entry function is.
entry_point!(kernel_main);

/// The main kernel entry point, called by the bootloader.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // =========================================================================
    // Phase 1: Core init (GDT → IDT → PIC+PIT, interrupts stay DISABLED)
    // =========================================================================
    florynx_kernel::init(boot_info);

    serial_println!("=========================================");
    serial_println!("  Florynx Kernel v0.2 — Booting...");
    serial_println!("=========================================");

    // =========================================================================
    // Phase 2: Memory initialization (still no interrupts)
    // =========================================================================
    serial_println!("[boot] phase 2: memory initialization...");

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);

    // Initialize paging
    let mut mapper = unsafe { florynx_kernel::memory::paging::init(phys_mem_offset) };
    serial_println!("[boot] page table initialized");

    // Initialize frame allocator from boot info memory map
    let mut frame_allocator = unsafe {
        florynx_kernel::memory::frame_allocator::BootInfoFrameAllocator::init(
            &boot_info.memory_map,
        )
    };
    serial_println!("[boot] frame allocator initialized");

    // Initialize the kernel heap
    florynx_kernel::memory::heap::init_heap(&mut mapper, &mut frame_allocator)
        .expect("heap initialization failed");
    serial_println!("[boot] heap initialized");

    // =========================================================================
    // Phase 3: GUI initialization (heap is ready, interrupts still disabled)
    // =========================================================================
    serial_println!("[boot] phase 3: GUI initialization...");
    florynx_kernel::init_gui(boot_info);

    // =========================================================================
    // Phase 4: Enable interrupts — ALL subsystems are now ready
    // =========================================================================
    serial_println!("[boot] phase 4: enabling interrupts...");
    x86_64::instructions::interrupts::enable();
    serial_println!("[boot] interrupts ENABLED");

    // =========================================================================
    // Phase 5: Post-init (banner, system info, launch desktop)
    // =========================================================================
    serial_println!("[boot] phase 5: post-init...");
    florynx_kernel::core_kernel::kernel::post_init();

    // Launch graphical desktop (mouse + windows + dock)
    florynx_kernel::launch_desktop();

    // =========================================================================
    // Phase 6: Stable halt loop with GUI redraw (60 FPS frame limiter)
    // =========================================================================
    serial_println!("[kernel] entering GUI hlt_loop (60 FPS)");
    
    const TARGET_FPS: u64 = 60;
    const TICKS_PER_FRAME: u64 = 200 / TARGET_FPS; // 200 Hz / 60 FPS = ~3 ticks per frame
    let mut last_frame_tick = florynx_kernel::time::clock::uptime_ticks();
    
    loop {
        x86_64::instructions::hlt();
        
        // Only redraw if enough time has passed (frame limiter)
        let current_tick = florynx_kernel::time::clock::uptime_ticks();
        if current_tick - last_frame_tick >= TICKS_PER_FRAME {
            florynx_kernel::gui::desktop::redraw_if_needed();
            last_frame_tick = current_tick;
        }
    }
}
