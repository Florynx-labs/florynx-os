// =============================================================================
// Florynx Kernel — execve() Implementation
// =============================================================================
// Loads a flat binary from the VFS into a fresh user address space and
// transfers execution to it, replacing the current process image.
//
// Binary layout in virtual memory:
//   CODE_BASE   = 0x0040_0000  (4 MiB) — binary mapped here, 1 page per sector
//   STACK_TOP   = 0x7FFF_F000  — top of user stack
//   STACK_PAGES = 4            — 16 KiB user stack
// =============================================================================

use alloc::vec::Vec;
use x86_64::structures::paging::{
    Mapper, Page, PageTableFlags, Size4KiB,
};
use x86_64::{VirtAddr, PhysAddr};

use crate::memory::{paging, frame_allocator};
use crate::memory::paging::PtFrameAlloc;
use crate::syscall::usermem;

pub const USER_CODE_BASE:  u64 = 0x0040_0000; // 4 MiB
pub const USER_STACK_TOP:  u64 = 0x7FFF_F000; // ~2 GiB
pub const USER_STACK_PAGES: usize = 4;

// Error codes
const EINVAL: i64 = -22;
const ENOMEM: i64 = -12;
const ENOENT: i64 = -2;
const ENOEXEC: i64 = -8;

/// `sys_execve(path_ptr, _argv, _envp)`
///
/// Reads a flat binary from the VFS, maps it at `USER_CODE_BASE` inside a
/// fresh page table, sets up a clean user stack, and jumps directly to
/// `USER_CODE_BASE` in ring 3.  This function **never returns** on success.
pub fn sys_execve(path_ptr: u64, _argv: u64, _envp: u64) -> i64 {
    // 1. Read the path from user space.
    let path = match usermem::read_cstr_from_user(path_ptr, 512) {
        Ok(p) => p,
        Err(e) => return e,
    };

    // 2. Load the binary bytes from VFS.
    let binary = load_binary(&path);
    let binary = match binary {
        Ok(b) => b,
        Err(e) => return e,
    };

    if binary.is_empty() {
        return ENOEXEC;
    }

    // 3. Build a fresh user page table (kernel mappings cloned from active L4).
    let phys_off = match paging::physical_memory_offset() {
        Some(o) => o,
        None => return ENOMEM,
    };

    let new_l4_frame = match unsafe {
        paging::create_user_page_table(&mut PtFrameAlloc, phys_off)
    } {
        Some(f) => f,
        None => return ENOMEM,
    };
    let new_cr3 = new_l4_frame.start_address();

    // 4. Map binary pages at USER_CODE_BASE inside the new page table.
    let mut mapper = unsafe { paging::init_from_l4_frame(new_l4_frame, phys_off) };
    let code_flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE; // will add NX later

    let pages_needed = (binary.len() + 0xFFF) / 0x1000;
    for i in 0..pages_needed {
        let virt = VirtAddr::new(USER_CODE_BASE + (i as u64) * 0x1000);
        let page: Page<Size4KiB> = Page::containing_address(virt);
        let frame = match frame_allocator::alloc_frame() {
            Some(f) => f,
            None => return ENOMEM,
        };
        unsafe {
            mapper
                .map_to(page, frame, code_flags, &mut PtFrameAlloc)
                .expect("exec: map_to code failed")
                .flush();
            // Copy binary slice for this page.
            let src_start = i * 0x1000;
            let src_end = core::cmp::min(src_start + 0x1000, binary.len());
            let dst = (phys_off.as_u64() + frame.start_address().as_u64()) as *mut u8;
            let count = src_end - src_start;
            core::ptr::copy_nonoverlapping(binary[src_start..src_end].as_ptr(), dst, count);
            // Zero any padding in the last page.
            if count < 0x1000 {
                core::ptr::write_bytes(dst.add(count), 0u8, 0x1000 - count);
            }
        }
    }

    // 5. Map user stack (USER_STACK_TOP - STACK_PAGES*4KiB .. USER_STACK_TOP).
    let stack_flags = PageTableFlags::PRESENT
        | PageTableFlags::USER_ACCESSIBLE
        | PageTableFlags::WRITABLE
        | PageTableFlags::NO_EXECUTE;

    let stack_base = USER_STACK_TOP - (USER_STACK_PAGES as u64) * 0x1000;
    for i in 0..USER_STACK_PAGES {
        let virt = VirtAddr::new(stack_base + (i as u64) * 0x1000);
        let page: Page<Size4KiB> = Page::containing_address(virt);
        let frame = match frame_allocator::alloc_frame() {
            Some(f) => f,
            None => return ENOMEM,
        };
        unsafe {
            mapper
                .map_to(page, frame, stack_flags, &mut PtFrameAlloc)
                .expect("exec: map_to stack failed")
                .flush();
            // Zero-fill stack pages.
            let dst = (phys_off.as_u64() + frame.start_address().as_u64()) as *mut u8;
            core::ptr::write_bytes(dst, 0u8, 0x1000);
        }
    }

    let entry     = VirtAddr::new(USER_CODE_BASE);
    let stack_top = VirtAddr::new(USER_STACK_TOP);

    crate::serial_println!(
        "[exec] loading '{}' ({} bytes, {} pages) entry=0x{:x} stack=0x{:x} cr3=0x{:x}",
        path, binary.len(), pages_needed,
        entry.as_u64(), stack_top.as_u64(), new_cr3.as_u64()
    );

    // 6. Update this task's process record to point at the new page table.
    update_process_page_table(new_cr3);

    // 7. Jump to user mode — never returns.
    unsafe {
        crate::process::task::jump_to_user_mode(entry, stack_top, Some(new_cr3), 0);
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Try to load a binary by path from the VFS (mounted backends first,
/// then the in-memory ramdisk via open+read).
fn load_binary(path: &str) -> Result<Vec<u8>, i64> {
    // 1. Try mounted backends (e.g. /disk/...).
    {
        let vfs = crate::fs::vfs::VFS.lock();
        if let Ok(data) = vfs.read_file_backend(path) {
            return Ok(data);
        }
    }

    // 2. Fall back to the in-memory VFS (ramdisk) via open+read.
    let fd = {
        let mut vfs = crate::fs::vfs::VFS.lock();
        match vfs.open(path, crate::fs::vfs::OpenFlags::read_only()) {
            Ok(desc) => desc.fd,
            Err(_) => return Err(ENOENT),
        }
    };

    let mut data: Vec<u8> = Vec::new();
    let mut offset = 0usize;
    loop {
        let mut vfs = crate::fs::vfs::VFS.lock();
        let mut chunk = [0u8; 4096];
        match vfs.read(fd, &mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&chunk[..n]);
                offset += n;
            }
            Err(_) => break,
        }
        if offset > 16 * 1024 * 1024 { break; } // 16 MiB safety cap
    }
    {
        let mut vfs = crate::fs::vfs::VFS.lock();
        let _ = vfs.close(fd);
    }

    if data.is_empty() { Err(ENOENT) } else { Ok(data) }
}

/// Replace the current task's associated Process record's page_table field
/// with the new CR3, and clear the old user_regions list.
fn update_process_page_table(new_cr3: PhysAddr) {
    use crate::process::scheduler::current_task_id;
    use crate::process::process::update_task_page_table;

    if let Some(tid) = current_task_id() {
        update_task_page_table(tid, new_cr3, true);
    }
}
