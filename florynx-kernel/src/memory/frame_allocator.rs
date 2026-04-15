// =============================================================================
// Florynx Kernel — Physical Frame Allocator
// =============================================================================
// Uses the bootloader-provided memory map to allocate physical frames.
// Iterates over usable memory regions and yields frames on demand.
// =============================================================================

use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};
use x86_64::PhysAddr;

lazy_static! {
    static ref FREED_FRAMES: Mutex<Vec<PhysFrame>> = Mutex::new(Vec::new());
}

/// A frame allocator that returns usable frames from the bootloader memory map.
///
/// O(1) bump allocator — tracks the current region and offset so each
/// allocation is a simple increment, not a full re-scan.
/// Never frees frames. A bitmap or buddy allocator should replace this
/// for production use.
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    /// Current address to allocate next (always 4KiB-aligned, within a usable region).
    next_addr: u64,
    /// Index into memory_map of the current usable region.
    region_idx: usize,
    /// Total allocated frame count (for diagnostics).
    allocated: usize,
}

impl BootInfoFrameAllocator {
    /// Create a new frame allocator from the bootloader memory map.
    ///
    /// # Safety
    /// The caller must guarantee that the memory map is valid and that all
    /// frames marked as `Usable` are truly unused.
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        let mut alloc = BootInfoFrameAllocator {
            memory_map,
            next_addr: 0,
            region_idx: 0,
            allocated: 0,
        };
        // Advance to the first usable region
        alloc.advance_to_usable();
        alloc
    }

    /// Advance `region_idx` and `next_addr` to the start of the next usable region.
    fn advance_to_usable(&mut self) {
        while self.region_idx < self.memory_map.len() {
            let region = &self.memory_map[self.region_idx];
            if region.region_type == MemoryRegionType::Usable {
                // If next_addr is before this region, jump to its start
                if self.next_addr < region.range.start_addr() {
                    self.next_addr = region.range.start_addr();
                }
                // If next_addr is still within this region, we're good
                if self.next_addr + 4096 <= region.range.end_addr() {
                    return;
                }
            }
            self.region_idx += 1;
        }
    }

    /// Returns the total number of usable frames (scans once).
    pub fn total_frames(&self) -> usize {
        self.memory_map
            .iter()
            .filter(|r| r.region_type == MemoryRegionType::Usable)
            .map(|r| ((r.range.end_addr() - r.range.start_addr()) / 4096) as usize)
            .sum()
    }

    /// Returns the total usable memory in bytes.
    pub fn total_memory(&self) -> u64 {
        self.total_frames() as u64 * 4096
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        if let Some(frame) = FREED_FRAMES.lock().pop() {
            return Some(frame);
        }
        if self.region_idx >= self.memory_map.len() {
            return None; // Out of memory
        }

        let frame = PhysFrame::containing_address(PhysAddr::new(self.next_addr));
        self.next_addr += 4096;
        self.allocated += 1;

        // If we've exhausted the current region, advance to the next usable one
        let region = &self.memory_map[self.region_idx];
        if self.next_addr >= region.range.end_addr() {
            self.region_idx += 1;
            self.advance_to_usable();
        }

        Some(frame)
    }
}

pub fn deallocate_frame(frame: PhysFrame) {
    FREED_FRAMES.lock().push(frame);
}

// ---------------------------------------------------------------------------
// Global frame allocator singleton — used by demand paging / guard page code.
// ---------------------------------------------------------------------------

struct GlobalFrameAlloc(Option<BootInfoFrameAllocator>);

// SAFETY: single-CPU kernel; the allocator is only accessed through the Mutex.
unsafe impl Send for GlobalFrameAlloc {}

lazy_static! {
    static ref GLOBAL_FRAME_ALLOC: Mutex<GlobalFrameAlloc> =
        Mutex::new(GlobalFrameAlloc(None));
}

/// Register the boot-time frame allocator as the global singleton.
/// Must be called exactly once from `kernel_main`, after the local allocator
/// has been initialised and the heap is ready.
pub fn init_global(alloc: BootInfoFrameAllocator) {
    let mut g = GLOBAL_FRAME_ALLOC.lock();
    g.0 = Some(alloc);
    crate::serial_println!("[frame_alloc] global frame allocator registered");
}

/// Allocate a single 4 KiB physical frame from the global allocator.
/// Returns `None` if OOM or if `init_global` was not yet called.
pub fn alloc_frame() -> Option<PhysFrame> {
    let mut g = GLOBAL_FRAME_ALLOC.lock();
    g.0.as_mut()?.allocate_frame()
}
