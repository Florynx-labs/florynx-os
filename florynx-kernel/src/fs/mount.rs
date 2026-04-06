// =============================================================================
// Florynx Kernel — Mount System
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;

/// A mounted filesystem entry.
#[derive(Debug)]
pub struct MountPoint {
    pub path: String,
    pub fs_type: String,
    pub device: String,
    pub read_only: bool,
}

/// Mount table tracking all mounted filesystems.
pub struct MountTable {
    mounts: Vec<MountPoint>,
}

impl MountTable {
    pub fn new() -> Self {
        MountTable {
            mounts: Vec::new(),
        }
    }

    pub fn mount(&mut self, path: &str, fs_type: &str, device: &str, read_only: bool) {
        self.mounts.push(MountPoint {
            path: String::from(path),
            fs_type: String::from(fs_type),
            device: String::from(device),
            read_only,
        });
    }

    pub fn umount(&mut self, path: &str) -> bool {
        if let Some(pos) = self.mounts.iter().position(|m| m.path == path) {
            self.mounts.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn find(&self, path: &str) -> Option<&MountPoint> {
        self.mounts.iter().find(|m| m.path == path)
    }

    pub fn list(&self) -> &[MountPoint] {
        &self.mounts
    }
}
