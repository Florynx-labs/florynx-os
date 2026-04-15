// =============================================================================
// Florynx Kernel — Virtual Memory Paging
// =============================================================================
// Initializes the page table by reading CR3 and creating an OffsetPageTable.
// The bootloader maps all physical memory at a configurable offset.
// =============================================================================

use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, Mapper, Page, PageTable, FrameAllocator, PhysFrame, Size4KiB, PageTableFlags};
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use lazy_static::lazy_static;

static PHYS_OFFSET: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Global mapper singleton — registered once by kernel_main.
// ---------------------------------------------------------------------------

struct GlobalMapper(Option<OffsetPageTable<'static>>);

// SAFETY: single-CPU kernel; only accessed through the Mutex.
unsafe impl Send for GlobalMapper {}

lazy_static! {
    static ref GLOBAL_MAPPER: Mutex<GlobalMapper> = Mutex::new(GlobalMapper(None));
}

/// Register the boot-time mapper as the global singleton.
/// Called once from `kernel_main` after `paging::init()` returns.
pub fn init_global_mapper(mapper: OffsetPageTable<'static>) {
    let mut g = GLOBAL_MAPPER.lock();
    g.0 = Some(mapper);
    crate::serial_println!("[paging] global mapper registered");
}


/// Initialize a new OffsetPageTable.
///
/// # Safety
/// The caller must guarantee that the complete physical memory is mapped to
/// virtual memory at the passed `physical_memory_offset`. Also, this function
/// must only be called once to avoid aliasing `&mut` references.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    PHYS_OFFSET.store(physical_memory_offset.as_u64(), Ordering::Relaxed);
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

#[inline]
pub fn physical_memory_offset() -> Option<VirtAddr> {
    let v = PHYS_OFFSET.load(Ordering::Relaxed);
    if v == 0 {
        None
    } else {
        Some(VirtAddr::new(v))
    }
}

/// Verify key kernel mappings are supervisor-only (not user accessible).
pub fn audit_kernel_supervisor_mappings() -> Result<(), &'static str> {
    let phys_offset = physical_memory_offset().ok_or("physical memory offset not initialized")?;
    let (l4_frame, _) = Cr3::read();
    let l4 = unsafe { page_table_from_phys(l4_frame.start_address().as_u64(), phys_offset.as_u64()) };

    // Higher-half entries must not be user accessible.
    for i in 256..512 {
        let e = &l4[i];
        if e.flags().contains(PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE) {
            return Err("kernel higher-half entry mapped user-accessible");
        }
    }

    // Ensure kernel heap base page is supervisor-only.
    let heap_addr = crate::memory::heap::HEAP_START as u64;
    if page_is_user_accessible(heap_addr, l4, phys_offset.as_u64())? {
        return Err("kernel heap is user-accessible");
    }

    Ok(())
}

fn page_is_user_accessible(vaddr: u64, l4: &PageTable, phys_offset: u64) -> Result<bool, &'static str> {
    let i4 = ((vaddr >> 39) & 0x1FF) as usize;
    let e4 = &l4[i4];
    let f4 = e4.flags();
    if !f4.contains(PageTableFlags::PRESENT) {
        return Err("address not mapped at L4");
    }
    if !f4.contains(PageTableFlags::USER_ACCESSIBLE) {
        return Ok(false);
    }
    let l3 = unsafe { page_table_from_phys(e4.addr().as_u64(), phys_offset) };

    let i3 = ((vaddr >> 30) & 0x1FF) as usize;
    let e3 = &l3[i3];
    let f3 = e3.flags();
    if !f3.contains(PageTableFlags::PRESENT) {
        return Err("address not mapped at L3");
    }
    if !f3.contains(PageTableFlags::USER_ACCESSIBLE) {
        return Ok(false);
    }
    if f3.contains(PageTableFlags::HUGE_PAGE) {
        return Ok(true);
    }
    let l2 = unsafe { page_table_from_phys(e3.addr().as_u64(), phys_offset) };

    let i2 = ((vaddr >> 21) & 0x1FF) as usize;
    let e2 = &l2[i2];
    let f2 = e2.flags();
    if !f2.contains(PageTableFlags::PRESENT) {
        return Err("address not mapped at L2");
    }
    if !f2.contains(PageTableFlags::USER_ACCESSIBLE) {
        return Ok(false);
    }
    if f2.contains(PageTableFlags::HUGE_PAGE) {
        return Ok(true);
    }
    let l1 = unsafe { page_table_from_phys(e2.addr().as_u64(), phys_offset) };

    let i1 = ((vaddr >> 12) & 0x1FF) as usize;
    let e1 = &l1[i1];
    let f1 = e1.flags();
    if !f1.contains(PageTableFlags::PRESENT) {
        return Err("address not mapped at L1");
    }
    Ok(f1.contains(PageTableFlags::USER_ACCESSIBLE))
}

unsafe fn page_table_from_phys(phys_addr: u64, phys_offset: u64) -> &'static PageTable {
    let virt = phys_offset + phys_addr;
    &*(virt as *const PageTable)
}

/// Returns a mutable reference to the active level-4 page table.
///
/// # Safety
/// The caller must guarantee that the complete physical memory is mapped to
/// virtual memory at the passed `physical_memory_offset`.
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

/// Create a new Level-4 page table for a user process.
/// It clones the kernel mappings (higher half) from the currently active table.
pub unsafe fn create_user_page_table(
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    physical_memory_offset: VirtAddr,
) -> Option<PhysFrame> {
    // 1. Allocate a frame for the new L4 table
    let new_table_frame = frame_allocator.allocate_frame()?;
    let virt = physical_memory_offset + new_table_frame.start_address().as_u64();
    let new_table_ptr: *mut PageTable = virt.as_mut_ptr();
    
    // 2. Clear the new table
    let new_table = unsafe { &mut *new_table_ptr };
    *new_table = PageTable::new();

    // 3. Copy ALL kernel mappings so that kernel code, stacks (TSS RSP0,
    //    IST) and the physical-memory window remain reachable when the CPU
    //    enters Ring 0 from Ring 3 through this page table.
    //    Lower-half kernel entries are supervisor-only, so Ring 3 cannot
    //    access them — no isolation is lost.
    let active_table = active_level_4_table(physical_memory_offset);
    for i in 0..512 {
        new_table[i] = active_table[i].clone();
    }

    Some(new_table_frame)
}

/// Map a freshly-allocated 4 KiB physical frame at `virt` with the given
/// flags using the global mapper and global frame allocator.
/// Returns `Err` if either singleton is uninitialised or OOM.
pub fn map_page_now(virt: VirtAddr, flags: PageTableFlags) -> Result<PhysFrame, &'static str> {
    let frame = crate::memory::frame_allocator::alloc_frame()
        .ok_or("OOM: no physical frame available")?;
    let mut gm = GLOBAL_MAPPER.lock();
    let mapper = gm.0.as_mut().ok_or("global mapper not initialised")?;
    let page: Page<Size4KiB> = Page::containing_address(virt);
    unsafe {
        mapper
            .map_to(page, frame, flags, &mut PtFrameAlloc)
            .map_err(|_| "map_to failed")?
            .flush();
    }
    Ok(frame)
}

/// Install a guard page at `virt`: ensure it is NOT present so any access
/// generates a page fault (stack-overflow / out-of-bounds detection).
/// If the page was previously mapped, it is unmapped and the frame freed.
pub fn install_guard_page(virt: VirtAddr) {
    let page: Page<Size4KiB> = Page::containing_address(virt);
    let mut gm = GLOBAL_MAPPER.lock();
    let mapper = match gm.0.as_mut() {
        Some(m) => m,
        None => return,
    };
    if let Ok((frame, flush)) = mapper.unmap(page) {
        flush.flush();
        crate::memory::frame_allocator::deallocate_frame(frame);
    }
    // If not mapped, already a guard page — nothing to do.
}

/// Adapter: allocates intermediate page-table frames from the global allocator.
pub struct PtFrameAlloc;
unsafe impl FrameAllocator<Size4KiB> for PtFrameAlloc {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        crate::memory::frame_allocator::alloc_frame()
    }
}

/// Build an OffsetPageTable view for an explicit L4 frame.
///
/// # Safety
/// Caller must ensure `l4_frame` is a valid level-4 page table frame and
/// physical memory is mapped at `physical_memory_offset`.
pub unsafe fn init_from_l4_frame(
    l4_frame: PhysFrame,
    physical_memory_offset: VirtAddr,
) -> OffsetPageTable<'static> {
    let virt = physical_memory_offset + l4_frame.start_address().as_u64();
    let l4_ptr: *mut PageTable = virt.as_mut_ptr();
    OffsetPageTable::new(&mut *l4_ptr, physical_memory_offset)
}
/// Recursively free all physical frames belonging to the user-space portion (0..256)
/// of the given level-4 page table. Does NOT free kernel-half mappings.
/// After walking the children, it deallocates the L4 frame itself.
///
/// # Safety
/// Caller must ensure `l4_frame` uniquely belongs to this process and is no longer active.
pub unsafe fn deep_cleanup_user_page_tables(l4_frame: PhysFrame) {
    let phys_offset = match physical_memory_offset() {
        Some(o) => o,
        None => return,
    };
    
    let l4_virt = phys_offset + l4_frame.start_address().as_u64();
    let l4_table: &mut PageTable = &mut *(l4_virt.as_mut_ptr());

    // Only iterate over the user-space half (0..256)
    for i in 0..256 {
        let entry = &mut l4_table[i];
        if entry.flags().contains(PageTableFlags::PRESENT) {
            let l3_frame = PhysFrame::containing_address(entry.addr());
            cleanup_l3_recursive(l3_frame, phys_offset);
            // After cleaning up the subtree, we don't zero the entry as the L4 frame is going away.
        }
    }

    // Finally, deallocate the L4 table frame itself.
    crate::memory::frame_allocator::deallocate_frame(l4_frame);
}

unsafe fn cleanup_l3_recursive(frame: PhysFrame, phys_offset: VirtAddr) {
    let table_virt = phys_offset + frame.start_address().as_u64();
    let table: &mut PageTable = &mut *(table_virt.as_mut_ptr());

    for i in 0..512 {
        let entry = &mut table[i];
        if entry.flags().contains(PageTableFlags::PRESENT) {
            if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                // It's a 1GB page - free the frame and continue
                crate::memory::frame_allocator::deallocate_frame(PhysFrame::containing_address(entry.addr()));
            } else {
                let l2_frame = PhysFrame::containing_address(entry.addr());
                cleanup_l2_recursive(l2_frame, phys_offset);
            }
        }
    }
    // Deallocate this L3 table frame
    crate::memory::frame_allocator::deallocate_frame(frame);
}

unsafe fn cleanup_l2_recursive(frame: PhysFrame, phys_offset: VirtAddr) {
    let table_virt = phys_offset + frame.start_address().as_u64();
    let table: &mut PageTable = &mut *(table_virt.as_mut_ptr());

    for i in 0..512 {
        let entry = &mut table[i];
        if entry.flags().contains(PageTableFlags::PRESENT) {
            if entry.flags().contains(PageTableFlags::HUGE_PAGE) {
                // It's a 2MB page - free the frame
                crate::memory::frame_allocator::deallocate_frame(PhysFrame::containing_address(entry.addr()));
            } else {
                let l1_frame = PhysFrame::containing_address(entry.addr());
                cleanup_l1_recursive(l1_frame, phys_offset);
            }
        }
    }
    // Deallocate this L2 table frame
    crate::memory::frame_allocator::deallocate_frame(frame);
}

unsafe fn cleanup_l1_recursive(frame: PhysFrame, phys_offset: VirtAddr) {
    let table_virt = phys_offset + frame.start_address().as_u64();
    let table: &mut PageTable = &mut *(table_virt.as_mut_ptr());

    for i in 0..512 {
        let entry = &mut table[i];
        if entry.flags().contains(PageTableFlags::PRESENT) {
            // It's a 4KB leaf frame - free it
            crate::memory::frame_allocator::deallocate_frame(PhysFrame::containing_address(entry.addr()));
        }
    }
    // Deallocate this L1 table frame
    crate::memory::frame_allocator::deallocate_frame(frame);
}
