// =============================================================================
// Florynx Kernel — fork() Implementation
// =============================================================================
// Clones the current user process:
//   1. Deep-copies all user-space pages (L4 lower half, indices 0..256).
//   2. Shares kernel mappings (upper half, indices 256..512) by reference.
//   3. Creates a child task that resumes at the syscall return point with rax=0.
//   4. Returns the child TaskId to the parent.
// =============================================================================

use x86_64::structures::paging::{PageTable, PageTableFlags};
use x86_64::{VirtAddr, PhysAddr};
use core::sync::atomic::Ordering;

use crate::arch::x86_64::idt::{SYSCALL_FRAME_RIP, SYSCALL_FRAME_RSP};
use crate::memory::{paging, frame_allocator};
use crate::process::scheduler;
use crate::security::capability::CapabilitySet;

// Error codes (mirroring syscall/handlers.rs)
const EINVAL: i64 = -22;
const ENOMEM: i64 = -12;
const ESRCH: i64  = -3;

/// Deep-copy the user-space page table of `parent_cr3`.
///
/// L4 entries 256–511 (kernel higher half) are shared by pointer — only the
/// lower-half user entries are duplicated frame-by-frame.
///
/// # Safety
/// Must be called from kernel mode with interrupts disabled or single-core context.
pub unsafe fn clone_user_page_table(parent_cr3: PhysAddr) -> Option<PhysAddr> {
    let phys_off = paging::physical_memory_offset()?.as_u64();

    // ---------- allocate new L4 ----------
    let new_l4_frame = frame_allocator::alloc_frame()?;
    let new_l4: &mut PageTable =
        &mut *((phys_off + new_l4_frame.start_address().as_u64()) as *mut PageTable);
    *new_l4 = PageTable::new();

    let parent_l4: &PageTable =
        &*((phys_off + parent_cr3.as_u64()) as *const PageTable);

    // Share kernel upper-half entries (256-511).
    for i in 256..512usize {
        new_l4[i] = parent_l4[i].clone();
    }

    // Deep-copy user lower-half entries (0-255).
    for i4 in 0..256usize {
        let e4 = &parent_l4[i4];
        if !e4.flags().contains(PageTableFlags::PRESENT) { continue; }
        if e4.flags().contains(PageTableFlags::HUGE_PAGE) {
            new_l4[i4] = e4.clone();
            continue;
        }

        let new_l3_frame = frame_allocator::alloc_frame()?;
        let new_l3: &mut PageTable =
            &mut *((phys_off + new_l3_frame.start_address().as_u64()) as *mut PageTable);
        *new_l3 = PageTable::new();
        new_l4[i4].set_addr(new_l3_frame.start_address(), e4.flags());

        let parent_l3: &PageTable =
            &*((phys_off + e4.addr().as_u64()) as *const PageTable);

        for i3 in 0..512usize {
            let e3 = &parent_l3[i3];
            if !e3.flags().contains(PageTableFlags::PRESENT) { continue; }
            if e3.flags().contains(PageTableFlags::HUGE_PAGE) {
                new_l3[i3] = e3.clone();
                continue;
            }

            let new_l2_frame = frame_allocator::alloc_frame()?;
            let new_l2: &mut PageTable =
                &mut *((phys_off + new_l2_frame.start_address().as_u64()) as *mut PageTable);
            *new_l2 = PageTable::new();
            new_l3[i3].set_addr(new_l2_frame.start_address(), e3.flags());

            let parent_l2: &PageTable =
                &*((phys_off + e3.addr().as_u64()) as *const PageTable);

            for i2 in 0..512usize {
                let e2 = &parent_l2[i2];
                if !e2.flags().contains(PageTableFlags::PRESENT) { continue; }
                if e2.flags().contains(PageTableFlags::HUGE_PAGE) {
                    new_l2[i2] = e2.clone();
                    continue;
                }

                let new_l1_frame = frame_allocator::alloc_frame()?;
                let new_l1: &mut PageTable =
                    &mut *((phys_off + new_l1_frame.start_address().as_u64()) as *mut PageTable);
                *new_l1 = PageTable::new();
                new_l2[i2].set_addr(new_l1_frame.start_address(), e2.flags());

                let parent_l1: &PageTable =
                    &*((phys_off + e2.addr().as_u64()) as *const PageTable);

                for i1 in 0..512usize {
                    let e1 = &parent_l1[i1];
                    if !e1.flags().contains(PageTableFlags::PRESENT) { continue; }

                    // Allocate new data frame and copy 4 KiB.
                    let new_data_frame = frame_allocator::alloc_frame()?;
                    let src = (phys_off + e1.addr().as_u64()) as *const u8;
                    let dst = (phys_off + new_data_frame.start_address().as_u64()) as *mut u8;
                    core::ptr::copy_nonoverlapping(src, dst, 4096);
                    new_l1[i1].set_addr(new_data_frame.start_address(), e1.flags());
                }
            }
        }
    }

    Some(new_l4_frame.start_address())
}

/// `sys_fork()` — creates a child process that is a copy of the current one.
///
/// Returns the child TaskId (> 0) in the parent, and 0 in the child (via
/// `initial_rax` in the child's `UserContext`).
pub fn sys_fork() -> i64 {
    // Read current task's user context via public scheduler API.
    let parent_cr3 = match scheduler::current_task_cr3() {
        Some(cr3) => cr3,
        None => return EINVAL,
    };

    // Use the precise RIP/RSP from the syscall iretq frame.
    let user_rip = VirtAddr::new(SYSCALL_FRAME_RIP.load(Ordering::Relaxed));
    let user_rsp = VirtAddr::new(SYSCALL_FRAME_RSP.load(Ordering::Relaxed));

    if user_rip.as_u64() == 0 {
        crate::serial_println!("[fork] user_rip=0, refusing to fork (syscall frame not set)");
        return EINVAL;
    }

    // Clone address space.
    let child_cr3 = match unsafe { clone_user_page_table(parent_cr3) } {
        Some(cr3) => cr3,
        None => return ENOMEM,
    };

    // Get parent capabilities.
    let caps = crate::process::scheduler::current_task_capabilities()
        .unwrap_or(CapabilitySet::user_default());

    // Spawn child: it will jump to user_rip with rax=0 on its first run.
    let child_id = crate::process::scheduler::spawn_fork_child(
        child_cr3,
        true,  // owns page table
        user_rip,
        user_rsp,
        caps,
    );

    crate::serial_println!(
        "[fork] parent tid={} spawned child tid={} at rip=0x{:x}",
        crate::process::scheduler::current_task_id().map(|t| t.0).unwrap_or(0),
        child_id.0,
        user_rip.as_u64()
    );

    child_id.0 as i64  // parent returns child PID
}
