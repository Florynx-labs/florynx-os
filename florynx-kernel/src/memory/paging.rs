// =============================================================================
// Florynx Kernel — Virtual Memory Paging
// =============================================================================
// Initializes the page table by reading CR3 and creating an OffsetPageTable.
// The bootloader maps all physical memory at a configurable offset.
// =============================================================================

use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable, FrameAllocator, PhysFrame, Size4KiB};
use x86_64::VirtAddr;

/// Initialize a new OffsetPageTable.
///
/// # Safety
/// The caller must guarantee that the complete physical memory is mapped to
/// virtual memory at the passed `physical_memory_offset`. Also, this function
/// must only be called once to avoid aliasing `&mut` references.
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
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
