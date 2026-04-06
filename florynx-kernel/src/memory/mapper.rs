// =============================================================================
// Florynx Kernel — Memory Mapping Utilities
// =============================================================================
// Convenience functions for mapping and unmapping virtual pages.
// =============================================================================

use x86_64::structures::paging::{
    FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};

/// Map a virtual page to a specific physical frame.
pub fn map_page(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    virtual_addr: VirtAddr,
    physical_addr: PhysAddr,
    flags: PageTableFlags,
) -> Result<(), &'static str> {
    let page = Page::containing_address(virtual_addr);
    let frame = PhysFrame::containing_address(physical_addr);

    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .map_err(|_| "failed to map page")?
            .flush();
    }

    Ok(())
}

/// Map a virtual page for user-space access.
/// Automatically sets the USER_ACCESSIBLE flag and disables execution if requested.
pub fn map_user_page(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
    virtual_addr: VirtAddr,
    physical_addr: PhysAddr,
    writable: bool,
    executable: bool,
) -> Result<(), &'static str> {
    let mut flags = PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE;
    
    if writable {
        flags |= PageTableFlags::WRITABLE;
    }
    
    if !executable {
        flags |= PageTableFlags::NO_EXECUTE;
    }

    map_page(mapper, frame_allocator, virtual_addr, physical_addr, flags)
}

/// Unmap a virtual page and return the physical frame it was mapped to.
pub fn unmap_page(
    mapper: &mut impl Mapper<Size4KiB>,
    virtual_addr: VirtAddr,
) -> Result<PhysFrame, &'static str> {
    let page: Page<Size4KiB> = Page::containing_address(virtual_addr);

    let (frame, flush) = mapper.unmap(page).map_err(|_| "failed to unmap page")?;
    flush.flush();

    Ok(frame)
}

/// Translate a virtual address to its mapped physical address.
pub fn translate_addr(
    mapper: &(impl Mapper<Size4KiB> + Translate),
    virtual_addr: VirtAddr,
) -> Option<PhysAddr> {
    mapper.translate_addr(virtual_addr)
}
