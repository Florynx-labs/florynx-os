// =============================================================================
// Florynx Kernel — Disk Driver Facade (Future)
// =============================================================================

use crate::drivers::Driver;

pub struct DiskDriver;

impl DiskDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Driver for DiskDriver {
    fn init(&mut self) {
        // Future: initialize AHCI/virtio-blk/NVMe backend.
    }

    fn handle_interrupt(&mut self) {
        // Future: complete async request and enqueue completion event.
    }

    fn update(&mut self) {
        // Future: submit deferred I/O work.
    }
}

