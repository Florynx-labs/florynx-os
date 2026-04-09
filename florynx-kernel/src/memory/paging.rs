// =============================================================================
// Florynx Kernel — Virtual Memory Paging
// =============================================================================
// Initializes the page table by reading CR3 and creating an OffsetPageTable.
// The bootloader maps all physical memory at a configurable offset.
// =============================================================================

use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable, FrameAllocator, PhysFrame, Size4KiB, PageTableFlags};
use x86_64::VirtAddr;
use core::sync::atomic::{AtomicU64, Ordering};

static PHYS_OFFSET: AtomicU64 = AtomicU64::new(0);

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

    // 3. Copy kernel mappings (entries 256..512 for x86_64 higher half)
    let active_table = active_level_4_table(physical_memory_offset);
    for i in 256..512 {
        new_table[i] = active_table[i].clone();
    }

    Some(new_table_frame)
}
