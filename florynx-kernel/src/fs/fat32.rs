// =============================================================================
// Florynx Kernel — FAT32 Read-Only Filesystem Driver
// =============================================================================
// Supports:
//   - FAT32 volumes (BPB validation, signature check)
//   - Short (8.3) filenames only
//   - File read via cluster chain traversal
//   - Directory listing
//   - Path resolution (absolute paths with '/' separator)
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;

use crate::drivers::block;
use super::vfs::{FsBackend, FileStat, FilePermissions, FileType, VfsError, VfsResult, DirEntry};

// ---------------------------------------------------------------------------
// BPB (BIOS Parameter Block) — first sector of the volume
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct Bpb {
    jmp_boot:        [u8; 3],
    oem_name:        [u8; 8],
    bytes_per_sector: u16,
    sectors_per_cluster: u8,
    reserved_sectors: u16,
    num_fats:        u8,
    root_entry_count: u16,  // 0 for FAT32
    total_sectors_16: u16,  // 0 for FAT32 if > 65535
    media:           u8,
    fat_size_16:     u16,   // 0 for FAT32
    sectors_per_track: u16,
    num_heads:       u16,
    hidden_sectors:  u32,
    total_sectors_32: u32,
    // FAT32-specific (offset +36)
    fat_size_32:     u32,
    ext_flags:       u16,
    fs_version:      u16,
    root_cluster:    u32,
    fs_info:         u16,
    backup_boot_sec: u16,
    _reserved:       [u8; 12],
    drive_number:    u8,
    _reserved1:      u8,
    boot_signature:  u8,
    volume_id:       u32,
    volume_label:    [u8; 11],
    fs_type_label:   [u8; 8],
}

// ---------------------------------------------------------------------------
// Directory entry (32 bytes)
// ---------------------------------------------------------------------------

#[repr(C, packed)]
#[derive(Clone, Copy)]
struct RawDirEntry {
    name:       [u8; 8],
    ext:        [u8; 3],
    attr:       u8,
    _nt:        u8,
    crt_time_tenth: u8,
    crt_time:   u16,
    crt_date:   u16,
    acc_date:   u16,
    clus_hi:    u16,  // high 16 bits of first cluster
    wrt_time:   u16,
    wrt_date:   u16,
    clus_lo:    u16,  // low  16 bits of first cluster
    file_size:  u32,
}

const ATTR_READ_ONLY: u8 = 0x01;
const ATTR_HIDDEN:    u8 = 0x02;
const ATTR_SYSTEM:    u8 = 0x04;
const ATTR_VOLUME_ID: u8 = 0x08;
const ATTR_DIRECTORY: u8 = 0x10;
const ATTR_ARCHIVE:   u8 = 0x20;
const ATTR_LFN:       u8 = ATTR_READ_ONLY | ATTR_HIDDEN | ATTR_SYSTEM | ATTR_VOLUME_ID;

impl RawDirEntry {
    fn is_free(&self)  -> bool { self.name[0] == 0x00 || self.name[0] == 0xE5 }
    fn is_lfn(&self)   -> bool { self.attr == ATTR_LFN }
    fn is_dir(&self)   -> bool { self.attr & ATTR_DIRECTORY != 0 }
    fn is_volume(&self)-> bool { self.attr & ATTR_VOLUME_ID != 0 }

    fn first_cluster(&self) -> u32 {
        ((self.clus_hi as u32) << 16) | (self.clus_lo as u32)
    }

    /// Parse the 8.3 name into a trimmed String like "KERNEL.BIN".
    fn short_name(&self) -> String {
        let base = self.name.iter()
            .take_while(|&&b| b != b' ' && b != 0)
            .map(|&b| b as char)
            .collect::<String>();
        let ext = self.ext.iter()
            .take_while(|&&b| b != b' ' && b != 0)
            .map(|&b| b as char)
            .collect::<String>();
        if ext.is_empty() { base } else { alloc::format!("{}.{}", base, ext) }
    }
}

// ---------------------------------------------------------------------------
// FAT32 driver
// ---------------------------------------------------------------------------

pub struct Fat32Fs {
    bytes_per_sector: u32,
    sectors_per_cluster: u32,
    fat_start_lba: u64,     // LBA of FAT0
    data_start_lba: u64,    // LBA of cluster 2
    root_cluster: u32,
    total_clusters: u32,
}

/// FAT32 end-of-chain markers (any value >= 0x0FFFFFF8).
const FAT_EOC: u32 = 0x0FFFFFF8;
/// FAT entry mask (28-bit values).
const FAT_MASK: u32 = 0x0FFFFFFF;

impl Fat32Fs {
    /// Mount a FAT32 volume from the registered block device.
    /// Reads sector 0, validates the BPB, and returns `None` if not FAT32.
    pub fn new() -> Option<Self> {
        let bps = block::block_size();
        if bps == 0 {
            crate::serial_println!("[fat32] no block device registered");
            return None;
        }

        // Read sector 0
        let mut sector = alloc::vec![0u8; bps];
        if block::read_blocks(0, 1, &mut sector).is_err() {
            crate::serial_println!("[fat32] failed to read sector 0");
            return None;
        }

        // Check boot signature
        if sector[510] != 0x55 || sector[511] != 0xAA {
            crate::serial_println!("[fat32] invalid boot signature");
            return None;
        }

        let bpb = unsafe { &*(sector.as_ptr() as *const Bpb) };
        let bytes_per_sector = bpb.bytes_per_sector as u32;
        let sectors_per_cluster = bpb.sectors_per_cluster as u32;

        if bytes_per_sector == 0 || sectors_per_cluster == 0 {
            crate::serial_println!("[fat32] invalid BPB geometry");
            return None;
        }

        // Validate FAT32: fat_size_16 must be 0, root_entry_count must be 0
        if bpb.fat_size_16 != 0 || bpb.root_entry_count != 0 {
            crate::serial_println!("[fat32] detected FAT12/16, not FAT32");
            return None;
        }

        // Check FS type label
        let fs_type = core::str::from_utf8(&bpb.fs_type_label).unwrap_or("").trim();
        if !fs_type.starts_with("FAT32") {
            crate::serial_println!("[fat32] fs_type='{}' — not FAT32", fs_type);
            return None;
        }

        let fat_size = bpb.fat_size_32;
        let fat_start_lba = bpb.reserved_sectors as u64;
        let data_start_lba = fat_start_lba
            + (bpb.num_fats as u64) * (fat_size as u64);

        let total_sectors = if bpb.total_sectors_32 != 0 {
            bpb.total_sectors_32 as u64
        } else {
            bpb.total_sectors_16 as u64
        };
        let data_sectors = total_sectors.saturating_sub(data_start_lba);
        let total_clusters = (data_sectors / sectors_per_cluster as u64) as u32;

        let root_clus = bpb.root_cluster; // copy out of packed struct
        crate::serial_println!(
            "[fat32] mounted: BPS={} SPC={} FAT@{} DATA@{} root_clus={} clusters={}",
            bytes_per_sector, sectors_per_cluster,
            fat_start_lba, data_start_lba,
            root_clus, total_clusters
        );

        Some(Fat32Fs {
            bytes_per_sector,
            sectors_per_cluster,
            fat_start_lba,
            data_start_lba,
            root_cluster: root_clus,
            total_clusters,
        })
    }

    // -----------------------------------------------------------------------
    // Internal helpers
    // -----------------------------------------------------------------------

    /// Read a single logical sector into `buf`.
    fn read_sector(&self, lba: u64, buf: &mut [u8]) -> VfsResult<()> {
        block::read_blocks(lba, 1, buf).map_err(|_| VfsError::IoError)
    }

    /// Read the FAT entry for `cluster`.
    fn fat_entry(&self, cluster: u32) -> VfsResult<u32> {
        // Each FAT32 entry is 4 bytes.
        let entries_per_sector = self.bytes_per_sector / 4;
        let sector_offset = cluster / entries_per_sector;
        let entry_offset  = (cluster % entries_per_sector) as usize;

        let lba = self.fat_start_lba + sector_offset as u64;
        let mut buf = alloc::vec![0u8; self.bytes_per_sector as usize];
        self.read_sector(lba, &mut buf)?;

        let val = u32::from_le_bytes([
            buf[entry_offset * 4],
            buf[entry_offset * 4 + 1],
            buf[entry_offset * 4 + 2],
            buf[entry_offset * 4 + 3],
        ]);
        Ok(val & FAT_MASK)
    }

    /// LBA of the first sector of `cluster`.
    fn cluster_lba(&self, cluster: u32) -> u64 {
        self.data_start_lba + (cluster as u64 - 2) * self.sectors_per_cluster as u64
    }

    /// Read the complete data for a cluster chain starting at `first_cluster`.
    /// Returns all the raw bytes (may include slack bytes in last cluster).
    fn read_chain(&self, first_cluster: u32) -> VfsResult<Vec<u8>> {
        let mut data = Vec::new();
        let mut cluster = first_cluster;
        let mut buf = alloc::vec![0u8; self.bytes_per_sector as usize];

        loop {
            if cluster < 2 || cluster >= FAT_EOC {
                break;
            }
            let lba = self.cluster_lba(cluster);
            for s in 0..self.sectors_per_cluster {
                self.read_sector(lba + s as u64, &mut buf)?;
                data.extend_from_slice(&buf);
            }
            let next = self.fat_entry(cluster)?;
            if next >= FAT_EOC || next < 2 {
                break;
            }
            cluster = next;
        }
        Ok(data)
    }

    /// Read all raw 32-byte directory entries in a directory cluster chain.
    fn read_dir_entries(&self, first_cluster: u32) -> VfsResult<Vec<RawDirEntry>> {
        let data = self.read_chain(first_cluster)?;
        let entry_size = core::mem::size_of::<RawDirEntry>();
        let count = data.len() / entry_size;
        let mut entries = Vec::new();
        for i in 0..count {
            let e = unsafe {
                *(data.as_ptr().add(i * entry_size) as *const RawDirEntry)
            };
            if e.name[0] == 0x00 {
                break; // no more entries
            }
            entries.push(e);
        }
        Ok(entries)
    }

    /// Resolve an absolute path to a `(first_cluster, file_size, is_dir)` tuple.
    /// Path must start with '/'.
    fn resolve(&self, path: &str) -> VfsResult<(u32, u32, bool)> {
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            return Ok((self.root_cluster, 0, true));
        }

        let mut current_cluster = self.root_cluster;
        let components: Vec<&str> = path.split('/').collect();

        for (i, component) in components.iter().enumerate() {
            let is_last = i == components.len() - 1;
            let upper = component.to_uppercase();
            let entries = self.read_dir_entries(current_cluster)?;

            let found = entries.iter().find(|e| {
                !e.is_free() && !e.is_lfn() && !e.is_volume()
                    && e.short_name().to_uppercase() == upper
            });

            match found {
                None => return Err(VfsError::NotFound),
                Some(e) => {
                    if is_last {
                        return Ok((e.first_cluster(), e.file_size, e.is_dir()));
                    } else if e.is_dir() {
                        current_cluster = e.first_cluster();
                    } else {
                        return Err(VfsError::NotADirectory);
                    }
                }
            }
        }
        Err(VfsError::NotFound)
    }
}

// ---------------------------------------------------------------------------
// FsBackend implementation
// ---------------------------------------------------------------------------

impl FsBackend for Fat32Fs {
    fn list_dir(&self, path: &str) -> VfsResult<Vec<DirEntry>> {
        let (cluster, _, is_dir) = self.resolve(path)?;
        if !is_dir {
            return Err(VfsError::NotADirectory);
        }
        let raw = self.read_dir_entries(cluster)?;
        let mut result = Vec::new();
        for e in raw {
            if e.is_free() || e.is_lfn() || e.is_volume() { continue; }
            let name = e.short_name();
            if name == "." || name == ".." { continue; }
            result.push(DirEntry {
                name,
                file_type: if e.is_dir() { FileType::Directory } else { FileType::Regular },
                size: e.file_size as u64,
            });
        }
        Ok(result)
    }

    fn read_file(&self, path: &str) -> VfsResult<Vec<u8>> {
        let (cluster, size, is_dir) = self.resolve(path)?;
        if is_dir {
            return Err(VfsError::IsADirectory);
        }
        let mut data = self.read_chain(cluster)?;
        data.truncate(size as usize);
        Ok(data)
    }

    fn stat(&self, path: &str) -> VfsResult<FileStat> {
        let (_, size, is_dir) = self.resolve(path)?;
        Ok(FileStat {
            inode: 0, // FAT32 has no inodes
            size: size as u64,
            file_type: if is_dir { FileType::Directory } else { FileType::Regular },
            permissions: FilePermissions::read_only(),
        })
    }
}
