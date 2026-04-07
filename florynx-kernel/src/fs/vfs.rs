// =============================================================================
// Florynx Kernel — Virtual File System (VFS)
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

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

/// File descriptor for open files
#[derive(Debug, Clone, Copy)]
pub struct FileDescriptor {
    pub fd: usize,
    pub inode: u64,
    pub offset: u64,
    pub flags: OpenFlags,
}

/// File open flags
#[derive(Debug, Clone, Copy)]
pub struct OpenFlags {
    pub read: bool,
    pub write: bool,
    pub create: bool,
    pub truncate: bool,
    pub append: bool,
}

impl OpenFlags {
    pub const fn read_only() -> Self {
        OpenFlags { read: true, write: false, create: false, truncate: false, append: false }
    }

    pub const fn write_only() -> Self {
        OpenFlags { read: false, write: true, create: false, truncate: false, append: false }
    }

    pub const fn read_write() -> Self {
        OpenFlags { read: true, write: true, create: false, truncate: false, append: false }
    }

    pub const fn create() -> Self {
        OpenFlags { read: true, write: true, create: true, truncate: false, append: false }
    }
}

/// VFS Error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VfsError {
    NotFound,
    AlreadyExists,
    NotADirectory,
    IsADirectory,
    PermissionDenied,
    InvalidPath,
    TooManyOpenFiles,
    InvalidFileDescriptor,
    EndOfFile,
}

pub type VfsResult<T> = Result<T, VfsError>;

/// The virtual filesystem.
pub struct Vfs {
    pub root: VfsNode,
    next_inode: u64,
    next_fd: usize,
    open_files: Vec<Option<FileDescriptor>>,
}

impl Vfs {
    pub fn new() -> Self {
        let mut root = VfsNode::new_directory("/", 0);
        
        // Create default directories
        root.add_child(VfsNode::new_directory("bin", 1));
        root.add_child(VfsNode::new_directory("etc", 2));
        root.add_child(VfsNode::new_directory("home", 3));
        root.add_child(VfsNode::new_directory("tmp", 4));
        root.add_child(VfsNode::new_directory("dev", 5));
        
        Vfs {
            root,
            next_inode: 6,
            next_fd: 3, // 0, 1, 2 reserved for stdin, stdout, stderr
            open_files: Vec::new(),
        }
    }

    pub fn root(&self) -> &VfsNode {
        &self.root
    }

    pub fn root_mut(&mut self) -> &mut VfsNode {
        &mut self.root
    }

    /// Allocate a new inode ID
    pub fn alloc_inode(&mut self) -> u64 {
        let id = self.next_inode;
        self.next_inode += 1;
        id
    }

    /// Allocate a new file descriptor
    fn alloc_fd(&mut self, inode: u64, flags: OpenFlags) -> VfsResult<FileDescriptor> {
        let fd = self.next_fd;
        self.next_fd += 1;
        
        let descriptor = FileDescriptor {
            fd,
            inode,
            offset: 0,
            flags,
        };
        
        // Store in open files table
        if self.open_files.len() <= fd {
            self.open_files.resize(fd + 1, None);
        }
        self.open_files[fd] = Some(descriptor);
        
        Ok(descriptor)
    }

    /// Open a file and return a file descriptor
    pub fn open(&mut self, path: &str, flags: OpenFlags) -> VfsResult<FileDescriptor> {
        // For now, simple path resolution (no nested paths)
        let node = self.find_node(path)?;
        
        if node.file_type == FileType::Directory {
            return Err(VfsError::IsADirectory);
        }
        
        if flags.write && !node.permissions.write {
            return Err(VfsError::PermissionDenied);
        }
        
        if flags.read && !node.permissions.read {
            return Err(VfsError::PermissionDenied);
        }
        
        self.alloc_fd(node.inode_id, flags)
    }

    /// Close a file descriptor
    pub fn close(&mut self, fd: usize) -> VfsResult<()> {
        if fd >= self.open_files.len() || self.open_files[fd].is_none() {
            return Err(VfsError::InvalidFileDescriptor);
        }
        
        self.open_files[fd] = None;
        Ok(())
    }

    /// Find a node by path
    fn find_node(&self, path: &str) -> VfsResult<&VfsNode> {
        if path == "/" {
            return Ok(&self.root);
        }
        
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        let mut current = &self.root;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            current = current.find_child(part).ok_or(VfsError::NotFound)?;
        }
        
        Ok(current)
    }

    /// Find a mutable node by path
    fn find_node_mut(&mut self, path: &str) -> VfsResult<&mut VfsNode> {
        if path == "/" {
            return Ok(&mut self.root);
        }
        
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        let mut current = &mut self.root;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            current = current.children.iter_mut()
                .find(|c| c.name == part)
                .ok_or(VfsError::NotFound)?;
        }
        
        Ok(current)
    }

    /// Create a new file
    pub fn create_file(&mut self, path: &str) -> VfsResult<()> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        
        let filename = parts[parts.len() - 1];
        let parent_path = if parts.len() == 1 {
            "/"
        } else {
            &path[..path.len() - filename.len() - 1]
        };
        
        // Allocate inode first
        let inode = self.alloc_inode();
        
        let parent = self.find_node_mut(parent_path)?;
        
        if parent.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }
        
        // Check if file already exists
        if parent.find_child(filename).is_some() {
            return Err(VfsError::AlreadyExists);
        }
        
        parent.add_child(VfsNode::new_file(filename, inode));
        
        Ok(())
    }

    /// Create a new directory
    pub fn create_dir(&mut self, path: &str) -> VfsResult<()> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.split('/').collect();
        
        if parts.is_empty() {
            return Err(VfsError::InvalidPath);
        }
        
        let dirname = parts[parts.len() - 1];
        let parent_path = if parts.len() == 1 {
            "/"
        } else {
            &path[..path.len() - dirname.len() - 1]
        };
        
        // Allocate inode first
        let inode = self.alloc_inode();
        
        let parent = self.find_node_mut(parent_path)?;
        
        if parent.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }
        
        // Check if directory already exists
        if parent.find_child(dirname).is_some() {
            return Err(VfsError::AlreadyExists);
        }
        
        parent.add_child(VfsNode::new_directory(dirname, inode));
        
        Ok(())
    }

    /// List directory contents
    pub fn list_dir(&self, path: &str) -> VfsResult<Vec<&VfsNode>> {
        let node = self.find_node(path)?;
        
        if node.file_type != FileType::Directory {
            return Err(VfsError::NotADirectory);
        }
        
        Ok(node.children.iter().collect())
    }

    /// Read from a file descriptor
    pub fn read(&mut self, fd: usize, buffer: &mut [u8]) -> VfsResult<usize> {
        if fd >= self.open_files.len() {
            return Err(VfsError::InvalidFileDescriptor);
        }
        
        let descriptor = self.open_files[fd].ok_or(VfsError::InvalidFileDescriptor)?;
        
        if !descriptor.flags.read {
            return Err(VfsError::PermissionDenied);
        }
        
        // Read from ramdisk
        let bytes_read = {
            let ramdisk = crate::fs::ramdisk::RAMDISK.lock();
            ramdisk.read(descriptor.inode, descriptor.offset as usize, buffer)
        };
        
        // Update offset
        if let Some(ref mut desc) = self.open_files[fd] {
            desc.offset += bytes_read as u64;
        }
        
        Ok(bytes_read)
    }

    /// Write to a file descriptor
    pub fn write(&mut self, fd: usize, data: &[u8]) -> VfsResult<usize> {
        if fd >= self.open_files.len() {
            return Err(VfsError::InvalidFileDescriptor);
        }
        
        let descriptor = self.open_files[fd].ok_or(VfsError::InvalidFileDescriptor)?;
        
        if !descriptor.flags.write {
            return Err(VfsError::PermissionDenied);
        }
        
        // Write to ramdisk
        let bytes_written = {
            let mut ramdisk = crate::fs::ramdisk::RAMDISK.lock();
            ramdisk.write(descriptor.inode, descriptor.offset as usize, data)
        };
        
        // Update offset and file size
        if let Some(ref mut desc) = self.open_files[fd] {
            desc.offset += bytes_written as u64;
        }
        
        // Update file size in VFS node
        if let Ok(node) = self.find_node_by_inode_mut(descriptor.inode) {
            let new_size = descriptor.offset + bytes_written as u64;
            if new_size > node.size {
                node.size = new_size;
            }
        }
        
        Ok(bytes_written)
    }

    /// Seek to a position in a file
    pub fn seek(&mut self, fd: usize, offset: u64) -> VfsResult<u64> {
        if fd >= self.open_files.len() {
            return Err(VfsError::InvalidFileDescriptor);
        }
        
        if let Some(ref mut descriptor) = self.open_files[fd] {
            descriptor.offset = offset;
            Ok(offset)
        } else {
            Err(VfsError::InvalidFileDescriptor)
        }
    }

    /// Find a mutable node by inode ID
    fn find_node_by_inode_mut(&mut self, inode: u64) -> VfsResult<&mut VfsNode> {
        Self::find_node_by_inode_recursive(&mut self.root, inode)
            .ok_or(VfsError::NotFound)
    }

    fn find_node_by_inode_recursive(node: &mut VfsNode, inode: u64) -> Option<&mut VfsNode> {
        if node.inode_id == inode {
            return Some(node);
        }
        
        for child in &mut node.children {
            if let Some(found) = Self::find_node_by_inode_recursive(child, inode) {
                return Some(found);
            }
        }
        
        None
    }

    /// Get file statistics
    pub fn stat(&self, path: &str) -> VfsResult<FileStat> {
        let node = self.find_node(path)?;
        Ok(FileStat {
            inode: node.inode_id,
            file_type: node.file_type,
            size: node.size,
            permissions: node.permissions,
        })
    }
}

/// File statistics
#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    pub inode: u64,
    pub file_type: FileType,
    pub size: u64,
    pub permissions: FilePermissions,
}

// =============================================================================
// Global VFS Instance
// =============================================================================

lazy_static! {
    /// Global VFS instance
    pub static ref VFS: Mutex<Vfs> = Mutex::new(Vfs::new());
}

/// Initialize the VFS
pub fn init() {
    let vfs = VFS.lock();
    crate::serial_println!("[vfs] initialized with {} directories", 
        vfs.root().children.len());
}
