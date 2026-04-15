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

use alloc::vec::Vec;
use bootloader::{entry_point, BootInfo};
use florynx_kernel::serial_println;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};
use x86_64::{PhysAddr, VirtAddr};

// Tell the bootloader crate where our entry function is.
entry_point!(kernel_main);

/// The main kernel entry point, called by the bootloader.
fn kernel_main(boot_info: &'static BootInfo) -> ! {
    // =========================================================================
    // Phase 1: Core init (GDT → IDT → PIC+PIT, interrupts stay DISABLED)
    // =========================================================================
    florynx_kernel::init(boot_info);

    serial_println!("╔═══════════════════════════════════════════════════════════════╗");
    serial_println!("║           Florynx Kernel v0.4.5 'Sentinel'                   ║");
    serial_println!("║     Production-Level Exception Handling • GUI • VFS          ║");
    serial_println!("╚═══════════════════════════════════════════════════════════════╝");

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

    // Register global frame allocator and mapper singletons (heap is ready).
    // These are used by guard pages and demand paging at runtime.
    florynx_kernel::memory::frame_allocator::init_global(unsafe {
        florynx_kernel::memory::frame_allocator::BootInfoFrameAllocator::init(
            &boot_info.memory_map,
        )
    });
    // Build a second OffsetPageTable view for the global mapper.
    let mapper2 = unsafe { florynx_kernel::memory::paging::init(phys_mem_offset) };
    florynx_kernel::memory::paging::init_global_mapper(mapper2);
    serial_println!("[boot] global memory singletons registered");

    // P0 isolation guard: kernel mappings must remain supervisor-only.
    florynx_kernel::memory::paging::audit_kernel_supervisor_mappings()
        .expect("kernel mapping isolation audit failed");
    serial_println!("[boot] kernel mapping isolation audit passed");

    // Initialize VFS, ramdisk, and devfs
    florynx_kernel::fs::ramdisk::init();
    florynx_kernel::fs::vfs::init();
    florynx_kernel::fs::devfs::init();

    // -------------------------------------------------------------------------
    // Phase 2: Block device discovery + FAT32 mount
    // -------------------------------------------------------------------------
    serial_println!("[boot] phase 2a: PCI block device discovery...");
    init_block_device();

    // Initialize scheduler
    florynx_kernel::process::scheduler::init();

    // Initialize syscall interface
    florynx_kernel::syscall::init();

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

    // Startup splash/logo (Windows-like boot experience).
    // 200 Hz PIT => ~240 ticks = ~1.2 seconds.
    florynx_kernel::gui::splash::show_startup_logo(240);

    // Launch HGUI compositor core (no kernel demo windows).
    florynx_kernel::launch_hgui();

    // Spawn a Ring3 launcher stub (boot-safe MVP) behind a scheduler task entry.
    if let Some((cr3, rip, rsp)) =
        build_ring3_launcher_process(phys_mem_offset, &mut frame_allocator)
    {
        florynx_kernel::process::scheduler::spawn_user_process(
            "hgui_launcher_ring3",
            cr3,
            true,
            rip,
            rsp,
            florynx_kernel::security::capability::CapabilitySet::user_default(),
        );
    } else {
        serial_println!("[boot] ring3 launcher stub setup failed; continuing without ring3 launcher");
    }

    // =========================================================================
    // Phase 6: Enable scheduler (HGUI handoff mode)
    // =========================================================================
    serial_println!("[boot] phase 6: enabling scheduler...");
    florynx_kernel::process::scheduler::enable();
    
    serial_println!("[boot] scheduler enabled with {} tasks", 
        florynx_kernel::process::scheduler::stats().total_tasks);
    run_validation_matrix();

    // =========================================================================
    // Phase 7: Stable halt loop with GUI redraw (60 FPS frame limiter)
    // =========================================================================
    serial_println!("[kernel] entering GUI hlt_loop (60 FPS)");
    
    const TARGET_FPS: u64 = 60;
    const TICKS_PER_FRAME: u64 = 200 / TARGET_FPS; // 200 Hz / 60 FPS = ~3 ticks per frame
    let mut last_frame_tick = florynx_kernel::time::clock::uptime_ticks();
    
    loop {
        x86_64::instructions::hlt();
        let _ = florynx_kernel::process::scheduler::run_current_user_first_run();
        florynx_kernel::drivers::process_events();
        florynx_kernel::drivers::update_deferred();
        
        // Only redraw if enough time has passed (frame limiter)
        let current_tick = florynx_kernel::time::clock::uptime_ticks();
        if current_tick - last_frame_tick >= TICKS_PER_FRAME {
            florynx_kernel::gui::desktop::redraw_if_needed();
            last_frame_tick = current_tick;
        }
    }
}

fn run_validation_matrix() {
    let sched = florynx_kernel::process::scheduler::stats();
    let cleanup = florynx_kernel::process::process::cleanup_telemetry();
    serial_println!(
        "[validate:A] tasks={} ready={} rounds={}",
        sched.total_tasks,
        sched.ready_tasks,
        sched.rounds
    );
    let faults = florynx_kernel::arch::x86_64::idt::fault_telemetry();
    let panic_t = florynx_kernel::core_kernel::panic::panic_telemetry();
    serial_println!(
        "[validate:B/C] syscall_ingress=int80 total_syscalls={}",
        faults.syscall_total
    );
    serial_println!(
        "[validate:D] cleanup_events={} fds={} links={} regions={} page_tables={}",
        cleanup.cleanup_events,
        cleanup.cleanup_fds_total,
        cleanup.cleanup_links_total,
        cleanup.cleanup_regions_total,
        cleanup.cleanup_page_tables_total
    );
    serial_println!(
        "[validate:E] pf_total={} pf_user={} pf_kernel={} panic_count={}",
        faults.page_fault_total,
        faults.page_fault_user,
        faults.page_fault_kernel,
        panic_t.panic_count
    );
}

fn build_ring3_launcher_process(
    physical_memory_offset: VirtAddr,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Option<(PhysAddr, VirtAddr, VirtAddr)> {
    const USER_CODE_VA: u64 = 0x0000_0000_4000_0000;
    const USER_STACK_VA: u64 = 0x0000_0000_4000_1000;
    const USER_STACK_TOP: u64 = USER_STACK_VA + 0x1000;

    let payload = load_ring3_launcher_payload();
    if payload.is_empty() {
        return None;
    }

    let user_l4 = unsafe {
        florynx_kernel::memory::paging::create_user_page_table(frame_allocator, physical_memory_offset)?
    };
    let mut user_mapper = unsafe {
        florynx_kernel::memory::paging::init_from_l4_frame(user_l4, physical_memory_offset)
    };

    let code_frame = frame_allocator.allocate_frame()?;
    let stack_frame = frame_allocator.allocate_frame()?;
    let code_phys = code_frame.start_address();
    let stack_phys = stack_frame.start_address();

    let code_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE;
    let stack_flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::USER_ACCESSIBLE | PageTableFlags::NO_EXECUTE;

    unsafe {
        user_mapper
            .map_to(
                Page::containing_address(VirtAddr::new(USER_CODE_VA)),
                code_frame,
                code_flags,
                frame_allocator,
            )
            .ok()?
            .flush();
        user_mapper
            .map_to(
                Page::containing_address(VirtAddr::new(USER_STACK_VA)),
                stack_frame,
                stack_flags,
                frame_allocator,
            )
            .ok()?
            .flush();
    }

    // Copy launcher code into mapped user code frame through direct phys mapping.
    if payload.len() > 4096 {
        serial_println!("[boot] launcher payload too large: {} bytes", payload.len());
        return None;
    }
    let code_ptr = (physical_memory_offset + code_phys.as_u64()).as_mut_ptr::<u8>();
    unsafe {
        core::ptr::copy_nonoverlapping(payload.as_ptr(), code_ptr, payload.len());
    }

    serial_println!(
        "[boot] ring3 launcher loaded code=0x{:x} stack=0x{:x} bytes={}",
        code_phys.as_u64(),
        stack_phys.as_u64(),
        payload.len()
    );
    Some((user_l4.start_address(), VirtAddr::new(USER_CODE_VA), VirtAddr::new(USER_STACK_TOP)))
}

fn load_ring3_launcher_payload() -> Vec<u8> {
    // Preferred path: payload from VFS/ramdisk for a real loader boundary.
    if let Some(vfs_payload) = read_payload_from_vfs("/bin/hgui_launcher.bin") {
        serial_println!("[boot] launcher payload source=vfs");
        return vfs_payload;
    }

    // Boot-safe fallback payload for bring-up.
    serial_println!("[boot] launcher payload source=fallback");
    let code: [u8; 24] = [
        0x48, 0xC7, 0xC0, 0x23, 0x00, 0x00, 0x00, // mov rax, 35 (SYS_SLEEP)
        0x48, 0xC7, 0xC7, 0x01, 0x00, 0x00, 0x00, // mov rdi, 1
        0x48, 0x31, 0xF6, // xor rsi, rsi
        0x48, 0x31, 0xD2, // xor rdx, rdx
        0xCD, 0x80, // int 0x80
        0xEB, 0xE8, // jmp start
    ];
    code.to_vec()
}

/// Probe PCI bus for a virtio-blk device, initialise it, and auto-mount
/// any FAT32 volume at `/disk` in the VFS.
fn init_block_device() {
    use florynx_kernel::drivers::pci;
    use florynx_kernel::drivers::block;
    use florynx_kernel::drivers::block::virtio_blk::VirtioBlk;
    use florynx_kernel::fs::fat32::Fat32Fs;
    use florynx_kernel::fs::vfs::VFS;

    // Step 1: enumerate PCI and find virtio-blk.
    let virtio = pci::find_virtio_blk();
    let pci_dev = match virtio {
        Some(d) => d,
        None => {
            serial_println!("[boot] no virtio-blk found on PCI bus — skipping disk mount");
            return;
        }
    };

    serial_println!(
        "[boot] found virtio-blk at {:02x}:{:02x}.{} BAR0=0x{:x}",
        pci_dev.bus, pci_dev.dev, pci_dev.func, pci_dev.bar0
    );

    // Step 2: extract I/O base from BAR0 (virtio legacy uses I/O space).
    let io_base = match pci_dev.bar0_io_base() {
        Some(b) => b,
        None => {
            serial_println!("[boot] virtio-blk BAR0 is not I/O space (0x{:x}), skipping", pci_dev.bar0);
            return;
        }
    };

    // Step 3: init driver and register as global block device.
    match VirtioBlk::init(io_base) {
        Some(dev) => block::register(dev),
        None => {
            serial_println!("[boot] virtio-blk init failed");
            return;
        }
    }

    // Step 4: try to detect and mount FAT32.
    match Fat32Fs::new() {
        Some(fs) => {
            VFS.lock().mount_backend("/disk", alloc::boxed::Box::new(fs));
            serial_println!("[boot] FAT32 volume mounted at /disk");
        }
        None => {
            serial_println!("[boot] disk present but not FAT32 — no filesystem mounted");
        }
    }
}

fn read_payload_from_vfs(path: &str) -> Option<Vec<u8>> {
    let mut vfs = florynx_kernel::fs::vfs::VFS.lock();
    let st = vfs.stat(path).ok()?;
    if st.size == 0 || st.size > 4096 {
        return None;
    }
    let fd = vfs
        .open(path, florynx_kernel::fs::vfs::OpenFlags::read_only())
        .ok()?;
    let mut buf = Vec::new();
    buf.resize(st.size as usize, 0);
    let n = vfs.read(fd.fd, &mut buf).ok()?;
    let _ = vfs.close(fd.fd);
    buf.truncate(n);
    Some(buf)
}

