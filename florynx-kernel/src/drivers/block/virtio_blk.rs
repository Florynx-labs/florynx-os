// =============================================================================
// Florynx Kernel — Virtio-blk Driver (Legacy I/O Port Interface)
// =============================================================================
// Implements the virtio 0.9.5 "legacy" block device specification.
// Uses I/O-port BAR0 and a single split-ring virtqueue (queue 0) with
// synchronous (polled) I/O — no IRQs required.
//
// Virtio legacy register map (relative to BAR0 I/O base):
//   +0x00  Device features (R, 32-bit)
//   +0x04  Guest features  (W, 32-bit)
//   +0x08  Queue PFN       (W, 32-bit) — physical page of virtqueue
//   +0x0C  Queue size      (R, 16-bit)
//   +0x0E  Queue select    (W, 16-bit)
//   +0x10  Queue notify    (W, 16-bit) — kick the device
//   +0x12  Device status   (RW, 8-bit)
//   +0x13  ISR status      (R, 8-bit)
// Device-specific config starts at +0x14:
//   +0x14  Capacity (sectors, u64)
// =============================================================================

use x86_64::instructions::port::Port;
use alloc::boxed::Box;
use alloc::vec::Vec;
use core::sync::atomic::{fence, Ordering};

use super::{BlockDevice, BlockError};

// ---------------------------------------------------------------------------
// Virtio device status bits
// ---------------------------------------------------------------------------
const STATUS_RESET:      u8 = 0x00;
const STATUS_ACK:        u8 = 0x01;
const STATUS_DRIVER:     u8 = 0x02;
const STATUS_DRIVER_OK:  u8 = 0x04;
const STATUS_FAILED:     u8 = 0x80;

// ---------------------------------------------------------------------------
// Virtqueue geometry
// ---------------------------------------------------------------------------
/// Number of descriptor slots in the virtqueue ring.
const QUEUE_SIZE: usize = 16;
/// Sector size (fixed by virtio-blk spec).
const SECTOR_SIZE: usize = 512;

// ---------------------------------------------------------------------------
// Virtio-blk request types
// ---------------------------------------------------------------------------
const VIRTIO_BLK_T_IN:  u32 = 0; // read
const VIRTIO_BLK_T_OUT: u32 = 1; // write

// ---------------------------------------------------------------------------
// Virtio split-ring virtqueue layout (packed into a single page-aligned Vec)
// ---------------------------------------------------------------------------

/// Virtqueue descriptor table entry (16 bytes).
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqDesc {
    addr:  u64,
    len:   u32,
    flags: u16,
    next:  u16,
}

const VIRTQ_DESC_F_NEXT:     u16 = 1;
const VIRTQ_DESC_F_WRITE:    u16 = 2; // device-writable (for in-direction)

/// Virtqueue available ring.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqAvail {
    flags: u16,
    idx:   u16,
    ring:  [u16; QUEUE_SIZE],
    used_event: u16,
}

/// One entry in the used ring.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqUsedElem {
    id:  u32,
    len: u32,
}

/// Virtqueue used ring.
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct VirtqUsed {
    flags: u16,
    idx:   u16,
    ring:  [VirtqUsedElem; QUEUE_SIZE],
    avail_event: u16,
}

// ---------------------------------------------------------------------------
// Virtio-blk request header (sent to device)
// ---------------------------------------------------------------------------
#[repr(C)]
#[derive(Clone, Copy)]
struct BlkReqHeader {
    type_:   u32,
    reserved: u32,
    sector:  u64,
}

// ---------------------------------------------------------------------------
// Virtqueue DMA region
// ---------------------------------------------------------------------------
/// One contiguous DMA buffer that holds the three virtqueue rings.
/// Must be allocated with 4 KiB alignment and physical addr == virtual addr
/// (possible because the kernel's direct physical mapping keeps virt = phys + offset,
/// but the bootloader maps physical memory 1:1 in the low region that QEMU uses
/// for virtio — typically below 4 GiB).
struct VirtqueueMem {
    /// Raw storage, must stay pinned.
    _storage: Vec<u8>,
    desc:  *mut VirtqDesc,
    avail: *mut VirtqAvail,
    used:  *mut VirtqUsed,
    /// Physical base address of the allocation.
    phys_base: u64,
}

// SAFETY: single-CPU kernel; all accesses go through the Mutex in the registry.
unsafe impl Send for VirtqueueMem {}

impl VirtqueueMem {
    /// Allocate and zero-init the virtqueue DMA region.
    fn new() -> Self {
        // Layout (must fit in one 4 KiB page):
        //   desc  table : 16 B × QUEUE_SIZE         = 256 B
        //   avail ring  : 6 + 2×QUEUE_SIZE + 2 B    = 40 B  (aligned +2)
        //   used  ring  : 6 + 8×QUEUE_SIZE + 2 B    = 136 B (page-aligned)
        // Total < 512 B, well within one 4096-byte page.
        let size = 4096;
        let mut storage: Vec<u8> = Vec::with_capacity(size + 4096);
        // Align to 4096
        let ptr = storage.as_ptr() as usize;
        let align_offset = if ptr % 4096 == 0 { 0 } else { 4096 - (ptr % 4096) };
        // Push enough bytes so we have size valid bytes starting at align_offset
        storage.resize(align_offset + size, 0u8);

        let base = storage.as_ptr() as usize + align_offset;
        let desc_ptr  = base as *mut VirtqDesc;
        let avail_ptr = (base + core::mem::size_of::<VirtqDesc>() * QUEUE_SIZE) as *mut VirtqAvail;
        let used_ptr  = (base + 2048) as *mut VirtqUsed; // start at offset 2048

        // Zero-init
        unsafe {
            core::ptr::write_bytes(base as *mut u8, 0, size);
        }

        // Physical address: for the bootloader-mapped kernel, the direct
        // physical map offset is stored in paging::physical_memory_offset().
        // virt = phys + offset  →  phys = virt - offset
        let phys_base = if let Some(offset) = crate::memory::paging::physical_memory_offset() {
            (base as u64).saturating_sub(offset.as_u64())
        } else {
            base as u64 // fallback: identity-mapped
        };

        VirtqueueMem {
            _storage: storage,
            desc:  desc_ptr,
            avail: avail_ptr,
            used:  used_ptr,
            phys_base,
        }
    }

    fn desc_phys(&self, idx: usize) -> u64 {
        self.phys_base + (idx * core::mem::size_of::<VirtqDesc>()) as u64
    }

    fn avail_phys(&self) -> u64 {
        self.phys_base + (core::mem::size_of::<VirtqDesc>() * QUEUE_SIZE) as u64
    }

    fn used_phys(&self) -> u64 {
        self.phys_base + 2048
    }
}

// ---------------------------------------------------------------------------
// Driver struct
// ---------------------------------------------------------------------------

pub struct VirtioBlk {
    io_base: u16,
    capacity: u64,    // sectors
    queue: VirtqueueMem,
    /// Next descriptor index to use (rotates 0..QUEUE_SIZE).
    desc_idx: usize,
    /// Shadow of avail.idx (how many we've added).
    avail_idx: u16,
    /// Shadow of last-seen used.idx.
    last_used_idx: u16,
}

impl VirtioBlk {
    /// Probe and initialise a virtio-blk device at the given I/O base port.
    /// Returns `None` if the device does not respond or negotiation fails.
    pub fn init(io_base: u16) -> Option<Box<dyn BlockDevice>> {
        crate::serial_println!("[virtio-blk] initialising at I/O 0x{:x}", io_base);

        let mut dev = VirtioBlk {
            io_base,
            capacity: 0,
            queue: VirtqueueMem::new(),
            desc_idx: 0,
            avail_idx: 0,
            last_used_idx: 0,
        };

        // Step 1: reset device
        dev.write_status(STATUS_RESET);
        // Step 2: ACK
        dev.write_status(STATUS_ACK);
        // Step 3: DRIVER
        dev.write_status(STATUS_ACK | STATUS_DRIVER);

        // Step 4: read & accept device features (accept all offered)
        let _device_features = dev.read_features();
        dev.write_guest_features(0); // minimal: request no optional features

        // Step 5: set up virtqueue 0
        dev.write_u16(0x0E, 0); // select queue 0
        let queue_size = dev.read_u16(0x0C) as usize;
        if queue_size == 0 {
            crate::serial_println!("[virtio-blk] queue size is 0, aborting");
            return None;
        }
        crate::serial_println!("[virtio-blk] queue size = {}", queue_size);

        // Write physical page frame number of the queue (4096-byte page)
        let pfn = (dev.queue.phys_base / 4096) as u32;
        crate::serial_println!("[virtio-blk] queue PFN = 0x{:x} (phys=0x{:x})", pfn, dev.queue.phys_base);
        dev.write_u32(0x08, pfn);

        // Step 6: DRIVER_OK
        dev.write_status(STATUS_ACK | STATUS_DRIVER | STATUS_DRIVER_OK);

        // Step 7: read capacity from device-specific config (+0x14, 8 bytes)
        let cap_lo = dev.read_u32(0x14);
        let cap_hi = dev.read_u32(0x18);
        dev.capacity = ((cap_hi as u64) << 32) | (cap_lo as u64);
        crate::serial_println!("[virtio-blk] capacity = {} sectors ({} MiB)",
            dev.capacity,
            dev.capacity * 512 / (1024 * 1024));

        Some(Box::new(dev))
    }

    // -----------------------------------------------------------------------
    // Register helpers
    // -----------------------------------------------------------------------

    fn read_u32(&self, offset: u16) -> u32 {
        let mut p: Port<u32> = Port::new(self.io_base + offset);
        unsafe { p.read() }
    }

    fn write_u32(&self, offset: u16, val: u32) {
        let mut p: Port<u32> = Port::new(self.io_base + offset);
        unsafe { p.write(val) }
    }

    fn read_u16(&self, offset: u16) -> u16 {
        let mut p: Port<u16> = Port::new(self.io_base + offset);
        unsafe { p.read() }
    }

    fn write_u16(&self, offset: u16, val: u16) {
        let mut p: Port<u16> = Port::new(self.io_base + offset);
        unsafe { p.write(val) }
    }

    fn read_u8(&self, offset: u16) -> u8 {
        let mut p: Port<u8> = Port::new(self.io_base + offset);
        unsafe { p.read() }
    }

    fn write_u8(&self, offset: u16, val: u8) {
        let mut p: Port<u8> = Port::new(self.io_base + offset);
        unsafe { p.write(val) }
    }

    fn read_features(&self) -> u32 { self.read_u32(0x00) }
    fn write_guest_features(&self, f: u32) { self.write_u32(0x04, f); }
    fn read_status(&self) -> u8 { self.read_u8(0x12) }
    fn write_status(&self, s: u8) { self.write_u8(0x12, s); }

    /// Kick queue 0 to notify the device.
    fn kick(&self) { self.write_u16(0x10, 0); }

    // -----------------------------------------------------------------------
    // Virtqueue submission + poll
    // -----------------------------------------------------------------------

    /// Submit one virtio-blk request and poll until the device completes it.
    ///
    /// Uses 3 descriptors:
    ///   [0] → BlkReqHeader   (device-readable)
    ///   [1] → data buffer    (device-readable for WRITE, device-writable for READ)
    ///   [2] → status byte    (device-writable)
    fn do_request(
        &mut self,
        req_type: u32,
        sector: u64,
        data: &mut [u8],
    ) -> Result<(), BlockError> {
        // Build request header in a stack buffer and get its physical address.
        let header = BlkReqHeader { type_: req_type, reserved: 0, sector };
        let status: u8 = 0xFF;

        // Physical addresses of each buffer component.
        let hdr_phys = virt_to_phys(&header as *const _ as u64);
        let dat_phys = virt_to_phys(data.as_ptr() as u64);
        let sta_phys = virt_to_phys(&status as *const _ as u64);

        // Descriptor indices (use a simple 3-slot window)
        let d0 = self.desc_idx % QUEUE_SIZE;
        let d1 = (self.desc_idx + 1) % QUEUE_SIZE;
        let d2 = (self.desc_idx + 2) % QUEUE_SIZE;

        unsafe {
            let desc = &mut *self.queue.desc;
            let desc_arr = core::slice::from_raw_parts_mut(desc as *mut VirtqDesc, QUEUE_SIZE);

            // Header descriptor
            desc_arr[d0] = VirtqDesc {
                addr:  hdr_phys,
                len:   core::mem::size_of::<BlkReqHeader>() as u32,
                flags: VIRTQ_DESC_F_NEXT,
                next:  d1 as u16,
            };

            // Data descriptor
            let data_flags = if req_type == VIRTIO_BLK_T_IN {
                VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT // device writes into it
            } else {
                VIRTQ_DESC_F_NEXT // device reads from it
            };
            desc_arr[d1] = VirtqDesc {
                addr:  dat_phys,
                len:   data.len() as u32,
                flags: data_flags,
                next:  d2 as u16,
            };

            // Status descriptor (always device-writable, no NEXT)
            desc_arr[d2] = VirtqDesc {
                addr:  sta_phys,
                len:   1,
                flags: VIRTQ_DESC_F_WRITE,
                next:  0,
            };

            // Write memory barrier before updating avail ring.
            fence(Ordering::Release);

            // Place head descriptor in avail ring.
            let avail = &mut *self.queue.avail;
            let slot = (self.avail_idx as usize) % QUEUE_SIZE;
            avail.ring[slot] = d0 as u16;
            fence(Ordering::Release);
            avail.idx = self.avail_idx.wrapping_add(1);
            self.avail_idx = avail.idx;

            fence(Ordering::Release);
        }

        self.desc_idx = (self.desc_idx + 3) % QUEUE_SIZE;

        // Kick device
        self.kick();

        // Poll used ring for completion (status byte turns 0 on success).
        let deadline = 1_000_000u32;
        for _ in 0..deadline {
            fence(Ordering::Acquire);
            let used_idx = unsafe { (*self.queue.used).idx };
            if used_idx != self.last_used_idx {
                self.last_used_idx = used_idx;
                break;
            }
            core::hint::spin_loop();
        }

        // Check status byte: 0 = OK, 1 = IOERR, 2 = UNSUPP
        match status {
            0 => Ok(()),
            _ => Err(BlockError::IoError),
        }
    }
}

impl BlockDevice for VirtioBlk {
    fn block_size(&self) -> usize { SECTOR_SIZE }
    fn block_count(&self) -> u64 { self.capacity }

    fn read_blocks(&mut self, lba: u64, count: usize, buf: &mut [u8]) -> Result<(), BlockError> {
        if buf.len() < SECTOR_SIZE * count {
            return Err(BlockError::BadBuffer);
        }
        if lba + count as u64 > self.capacity {
            return Err(BlockError::OutOfBounds);
        }
        for i in 0..count {
            let slice = &mut buf[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE];
            self.do_request(VIRTIO_BLK_T_IN, lba + i as u64, slice)?;
        }
        Ok(())
    }

    fn write_blocks(&mut self, lba: u64, count: usize, buf: &[u8]) -> Result<(), BlockError> {
        if buf.len() < SECTOR_SIZE * count {
            return Err(BlockError::BadBuffer);
        }
        if lba + count as u64 > self.capacity {
            return Err(BlockError::OutOfBounds);
        }
        for i in 0..count {
            let mut sector_buf = [0u8; SECTOR_SIZE];
            sector_buf.copy_from_slice(&buf[i * SECTOR_SIZE..(i + 1) * SECTOR_SIZE]);
            self.do_request(VIRTIO_BLK_T_OUT, lba + i as u64, &mut sector_buf)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Helper: virtual → physical address translation
// ---------------------------------------------------------------------------

/// Translate a kernel virtual address to its physical address using the
/// stored physical memory offset.  Falls back to identity mapping if the
/// offset is not yet set (should not happen after Phase 2 boot).
fn virt_to_phys(virt: u64) -> u64 {
    if let Some(offset) = crate::memory::paging::physical_memory_offset() {
        virt.saturating_sub(offset.as_u64())
    } else {
        virt
    }
}
