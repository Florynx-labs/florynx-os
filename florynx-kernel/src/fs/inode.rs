// =============================================================================
// Florynx Kernel — Inode Structure
// =============================================================================

/// Inode — the on-disk representation of a file.
#[derive(Debug, Clone)]
pub struct Inode {
    pub id: u64,
    pub file_type: u8,
    pub permissions: u16,
    pub owner: u32,
    pub group: u32,
    pub size: u64,
    pub block_count: u64,
    pub created_at: u64,
    pub modified_at: u64,
    pub accessed_at: u64,
    /// Direct block pointers.
    pub direct_blocks: [u64; 12],
    /// Single indirect block pointer.
    pub indirect_block: u64,
    /// Double indirect block pointer.
    pub double_indirect_block: u64,
}

impl Inode {
    pub fn new(id: u64) -> Self {
        Inode {
            id,
            file_type: 0,
            permissions: 0o644,
            owner: 0,
            group: 0,
            size: 0,
            block_count: 0,
            created_at: 0,
            modified_at: 0,
            accessed_at: 0,
            direct_blocks: [0; 12],
            indirect_block: 0,
            double_indirect_block: 0,
        }
    }
}
