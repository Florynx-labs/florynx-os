// =============================================================================
// Florynx Kernel — Block Device Abstraction
// =============================================================================
// Defines the BlockDevice trait and a single global block-device registry.
// Drivers (virtio-blk, ATA, NVMe) implement BlockDevice and register
// themselves so the filesystem layer can read/write sectors without knowing
// which hardware backend is active.
// =============================================================================

pub mod virtio_blk;

use alloc::boxed::Box;
use spin::Mutex;
use lazy_static::lazy_static;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockError {
    /// LBA out of range.
    OutOfBounds,
    /// Buffer length not a multiple of block_size * count.
    BadBuffer,
    /// Hardware / driver error.
    IoError,
    /// Driver not initialised yet.
    NotReady,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Abstraction over any block-addressable storage device.
///
/// All I/O is synchronous (polled) for now.  Async completion will be added
/// once we have a proper interrupt-driven I/O path.
pub trait BlockDevice: Send {
    /// Size of one logical block in bytes (almost always 512).
    fn block_size(&self) -> usize;

    /// Total number of logical blocks on the device.
    fn block_count(&self) -> u64;

    /// Read `count` consecutive blocks starting at `lba` into `buf`.
    /// `buf.len()` must be >= `block_size() * count`.
    fn read_blocks(&mut self, lba: u64, count: usize, buf: &mut [u8]) -> Result<(), BlockError>;

    /// Write `count` consecutive blocks starting at `lba` from `buf`.
    /// `buf.len()` must be >= `block_size() * count`.
    fn write_blocks(&mut self, lba: u64, count: usize, buf: &[u8]) -> Result<(), BlockError>;
}

// ---------------------------------------------------------------------------
// Global singleton — primary block device (e.g. virtio-blk disk 0)
// ---------------------------------------------------------------------------

struct GlobalBlockDev(Option<Box<dyn BlockDevice>>);

// SAFETY: single-CPU kernel, protected by Mutex.
unsafe impl Send for GlobalBlockDev {}

lazy_static! {
    static ref BLOCK_DEV: Mutex<GlobalBlockDev> = Mutex::new(GlobalBlockDev(None));
}

/// Register a block device as the primary storage backend.
/// Replaces any previously registered device.
pub fn register(dev: Box<dyn BlockDevice>) {
    let mut g = BLOCK_DEV.lock();
    crate::serial_println!(
        "[block] registered device: {} blocks × {} B = {} MiB",
        dev.block_count(),
        dev.block_size(),
        dev.block_count() * dev.block_size() as u64 / (1024 * 1024)
    );
    g.0 = Some(dev);
}

/// Returns `true` if a block device has been registered.
pub fn is_ready() -> bool {
    BLOCK_DEV.lock().0.is_some()
}

/// Read `count` blocks from the primary block device.
pub fn read_blocks(lba: u64, count: usize, buf: &mut [u8]) -> Result<(), BlockError> {
    let mut g = BLOCK_DEV.lock();
    match g.0.as_mut() {
        Some(dev) => dev.read_blocks(lba, count, buf),
        None => Err(BlockError::NotReady),
    }
}

/// Write `count` blocks to the primary block device.
pub fn write_blocks(lba: u64, count: usize, buf: &[u8]) -> Result<(), BlockError> {
    let mut g = BLOCK_DEV.lock();
    match g.0.as_mut() {
        Some(dev) => dev.write_blocks(lba, count, buf),
        None => Err(BlockError::NotReady),
    }
}

/// Block size of the primary device (0 if none registered).
pub fn block_size() -> usize {
    BLOCK_DEV.lock().0.as_ref().map(|d| d.block_size()).unwrap_or(0)
}

/// Block count of the primary device (0 if none registered).
pub fn block_count() -> u64 {
    BLOCK_DEV.lock().0.as_ref().map(|d| d.block_count()).unwrap_or(0)
}
