// =============================================================================
// Florynx Kernel — Kernel Heap Allocator
// =============================================================================
// Maps virtual pages for the kernel heap and initializes the linked-list
// allocator. The heap lives at a fixed virtual address range.
// =============================================================================

use linked_list_allocator::LockedHeap;
use x86_64::structures::paging::{
    mapper::MapToError, FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB,
};
use x86_64::VirtAddr;

/// Start address of the kernel heap (chosen to be in a high unmapped region).
pub const HEAP_START: usize = 0x_4444_4444_0000;
/// Size of the kernel heap: 16 MiB (double buffer ~3 MiB + bg cache ~2.3 MiB + general).
pub const HEAP_SIZE: usize = 16 * 1024 * 1024;

/// Global kernel heap allocator (linked-list based).
#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

/// Initialize the kernel heap by mapping pages and setting up the allocator.
pub fn init_heap(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map each heap page to a physical frame
    for page in page_range {
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    // Initialize the allocator with the heap memory range
    unsafe {
        ALLOCATOR.lock().init(HEAP_START, HEAP_SIZE);
    }

    crate::serial_println!(
        "[heap] initialized at {:#x}, size {} KiB",
        HEAP_START,
        HEAP_SIZE / 1024
    );

    Ok(())
}
