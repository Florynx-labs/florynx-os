// =============================================================================
// Florynx Kernel — Ramdisk Filesystem Driver
// =============================================================================
// In-memory filesystem for storing file data
// =============================================================================

use alloc::collections::BTreeMap;
use spin::Mutex;

const BLOCK_SIZE: usize = 4096;
const MAX_BLOCKS: usize = 1024; // 4 MiB total

/// A block of data in the ramdisk
type Block = [u8; BLOCK_SIZE];

/// Ramdisk storage
pub struct Ramdisk {
    blocks: BTreeMap<u64, Block>,  // inode -> block mapping
    next_block: u64,
}

impl Ramdisk {
    pub const fn new() -> Self {
        Ramdisk {
            blocks: BTreeMap::new(),
            next_block: 0,
        }
    }

    /// Allocate a new block for an inode
    pub fn alloc_block(&mut self, inode: u64) -> Option<u64> {
        if self.blocks.len() >= MAX_BLOCKS {
            return None;
        }

        let block_id = self.next_block;
        self.next_block += 1;
        
        self.blocks.insert(inode, [0u8; BLOCK_SIZE]);
        Some(block_id)
    }

    /// Read data from an inode's block
    pub fn read(&self, inode: u64, offset: usize, buffer: &mut [u8]) -> usize {
        if let Some(block) = self.blocks.get(&inode) {
            let start = offset.min(BLOCK_SIZE);
            let end = (offset + buffer.len()).min(BLOCK_SIZE);
            let len = end.saturating_sub(start);
            
            if len > 0 {
                buffer[..len].copy_from_slice(&block[start..end]);
                return len;
            }
        }
        0
    }

    /// Write data to an inode's block
    pub fn write(&mut self, inode: u64, offset: usize, data: &[u8]) -> usize {
        // Allocate block if it doesn't exist
        if !self.blocks.contains_key(&inode) {
            if self.alloc_block(inode).is_none() {
                return 0; // Out of space
            }
        }

        if let Some(block) = self.blocks.get_mut(&inode) {
            let start = offset.min(BLOCK_SIZE);
            let end = (offset + data.len()).min(BLOCK_SIZE);
            let len = end.saturating_sub(start);
            
            if len > 0 {
                block[start..end].copy_from_slice(&data[..len]);
                return len;
            }
        }
        0
    }

    /// Get the size of data stored for an inode
    pub fn get_size(&self, inode: u64) -> usize {
        if self.blocks.contains_key(&inode) {
            BLOCK_SIZE
        } else {
            0
        }
    }

    /// Clear all data for an inode
    pub fn clear(&mut self, inode: u64) {
        self.blocks.remove(&inode);
    }

    /// Get total blocks used
    pub fn blocks_used(&self) -> usize {
        self.blocks.len()
    }

    /// Get total blocks available
    pub fn blocks_total(&self) -> usize {
        MAX_BLOCKS
    }
}

/// Global ramdisk instance
pub static RAMDISK: Mutex<Ramdisk> = Mutex::new(Ramdisk::new());

/// Initialize the ramdisk
pub fn init() {
    crate::serial_println!("[ramdisk] initialized ({} KiB capacity)", 
        (MAX_BLOCKS * BLOCK_SIZE) / 1024);
}
