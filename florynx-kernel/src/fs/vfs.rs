// =============================================================================
// Florynx Kernel — Virtual File System (VFS)
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;

/// File type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    SymLink,
    Device,
    Pipe,
}

/// File permissions.
#[derive(Debug, Clone, Copy)]
pub struct FilePermissions {
    pub read: bool,
    pub write: bool,
    pub execute: bool,
}

impl FilePermissions {
    pub const fn read_only() -> Self {
        FilePermissions { read: true, write: false, execute: false }
    }

    pub const fn read_write() -> Self {
        FilePermissions { read: true, write: true, execute: false }
    }

    pub const fn all() -> Self {
        FilePermissions { read: true, write: true, execute: true }
    }
}

/// VFS node representing a file or directory.
pub struct VfsNode {
    pub name: String,
    pub file_type: FileType,
    pub permissions: FilePermissions,
    pub size: u64,
    pub inode_id: u64,
    pub children: Vec<VfsNode>,
}

impl VfsNode {
    pub fn new_file(name: &str, inode_id: u64) -> Self {
        VfsNode {
            name: String::from(name),
            file_type: FileType::Regular,
            permissions: FilePermissions::read_write(),
            size: 0,
            inode_id,
            children: Vec::new(),
        }
    }

    pub fn new_directory(name: &str, inode_id: u64) -> Self {
        VfsNode {
            name: String::from(name),
            file_type: FileType::Directory,
            permissions: FilePermissions::all(),
            size: 0,
            inode_id,
            children: Vec::new(),
        }
    }

    pub fn add_child(&mut self, child: VfsNode) {
        self.children.push(child);
    }

    pub fn find_child(&self, name: &str) -> Option<&VfsNode> {
        self.children.iter().find(|c| c.name == name)
    }
}

/// The virtual filesystem.
pub struct Vfs {
    pub root: VfsNode,
}

impl Vfs {
    pub fn new() -> Self {
        Vfs {
            root: VfsNode::new_directory("/", 0),
        }
    }

    pub fn root(&self) -> &VfsNode {
        &self.root
    }
}
