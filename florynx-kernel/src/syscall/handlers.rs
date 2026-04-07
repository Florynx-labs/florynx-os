// =============================================================================
// Florynx Kernel — Syscall Handlers
// =============================================================================
// Production-level syscall handler implementations.
// Integrates with VFS, scheduler, and I/O subsystems.
// =============================================================================

use crate::fs::vfs::{VFS, OpenFlags};
use crate::process::scheduler;

// Error codes (POSIX-compatible)
const EBADF: i64 = -9;
const EINVAL: i64 = -22;
const ENOENT: i64 = -2;
const EACCES: i64 = -13;
const EISDIR: i64 = -21;
const EEXIST: i64 = -17;
const EFAULT: i64 = -14;

/// sys_write — write data to a file descriptor.
/// fd=0: stdin (invalid for write)
/// fd=1: stdout → serial + VGA
/// fd=2: stderr → serial
/// fd>=3: VFS file descriptor
pub fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> i64 {
    if buf_ptr == 0 || len == 0 {
        return EFAULT;
    }
    
    let buf = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len as usize)
    };

    match fd {
        0 => EBADF, // stdin not writable
        1 => {
            // stdout → serial + VGA
            if let Ok(s) = core::str::from_utf8(buf) {
                crate::serial_print!("{}", s);
                crate::print!("{}", s);
            }
            len as i64
        }
        2 => {
            // stderr → serial only
            if let Ok(s) = core::str::from_utf8(buf) {
                crate::serial_print!("{}", s);
            }
            len as i64
        }
        fd => {
            // VFS file descriptor
            let mut vfs = VFS.lock();
            match vfs.write(fd as usize, buf) {
                Ok(n) => n as i64,
                Err(_) => EBADF,
            }
        }
    }
}

/// sys_read — read data from a file descriptor.
/// fd=0: stdin (keyboard buffer)
/// fd>=3: VFS file descriptor
pub fn sys_read(fd: u64, buf_ptr: u64, len: u64) -> i64 {
    if buf_ptr == 0 || len == 0 {
        return EFAULT;
    }
    
    let buf = unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut u8, len as usize)
    };

    match fd {
        0 => {
            // stdin — currently no keyboard buffer, return 0
            0
        }
        1 | 2 => EBADF, // stdout/stderr not readable
        fd => {
            // VFS file descriptor
            let mut vfs = VFS.lock();
            match vfs.read(fd as usize, buf) {
                Ok(n) => n as i64,
                Err(_) => EBADF,
            }
        }
    }
}

/// sys_open — open a file by path.
/// flags: 0=read, 1=write, 2=read+write, 4=create
pub fn sys_open(path_ptr: u64, path_len: u64, flags: u64) -> i64 {
    if path_ptr == 0 {
        return EFAULT;
    }
    
    let path_bytes = unsafe {
        core::slice::from_raw_parts(path_ptr as *const u8, path_len as usize)
    };
    
    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => return EINVAL,
    };
    
    let open_flags = match flags {
        0 => OpenFlags::read_only(),
        1 => OpenFlags::write_only(),
        2 => OpenFlags::read_write(),
        4 => OpenFlags::create(),
        _ => return EINVAL,
    };
    
    // If create flag, create file first if it doesn't exist
    if open_flags.create {
        let mut vfs = VFS.lock();
        let _ = vfs.create_file(path); // Ignore AlreadyExists error
        drop(vfs);
    }
    
    let mut vfs = VFS.lock();
    match vfs.open(path, open_flags) {
        Ok(fd) => fd.fd as i64,
        Err(crate::fs::vfs::VfsError::NotFound) => ENOENT,
        Err(crate::fs::vfs::VfsError::PermissionDenied) => EACCES,
        Err(crate::fs::vfs::VfsError::IsADirectory) => EISDIR,
        Err(crate::fs::vfs::VfsError::AlreadyExists) => EEXIST,
        Err(_) => EINVAL,
    }
}

/// sys_close — close a file descriptor.
pub fn sys_close(fd: u64) -> i64 {
    if fd < 3 {
        return EBADF; // Can't close stdin/stdout/stderr
    }
    
    let mut vfs = VFS.lock();
    match vfs.close(fd as usize) {
        Ok(()) => 0,
        Err(_) => EBADF,
    }
}

/// sys_seek — seek to position in file.
/// whence: 0=SET, 1=CUR (not implemented), 2=END (not implemented)
pub fn sys_seek(fd: u64, offset: u64, _whence: u64) -> i64 {
    if fd < 3 {
        return EBADF;
    }
    
    let mut vfs = VFS.lock();
    match vfs.seek(fd as usize, offset) {
        Ok(pos) => pos as i64,
        Err(_) => EBADF,
    }
}

/// sys_exit — terminate the current process.
pub fn sys_exit(exit_code: u64) -> i64 {
    crate::serial_println!("[syscall] exit with code {}", exit_code);
    scheduler::exit();
    0
}

/// sys_yield — yield the CPU to the scheduler.
pub fn sys_yield() -> i64 {
    scheduler::yield_now();
    0
}

/// sys_getpid — return the current process ID.
pub fn sys_getpid() -> i64 {
    match scheduler::current_task_id() {
        Some(id) => id.0 as i64,
        None => 0,
    }
}

/// sys_sleep — sleep for a given number of ticks.
pub fn sys_sleep(ticks: u64) -> i64 {
    let start = crate::drivers::timer::pit::get_ticks();
    while crate::drivers::timer::pit::get_ticks() - start < ticks {
        x86_64::instructions::hlt();
    }
    0
}

/// sys_mkdir — create a directory.
pub fn sys_mkdir(path_ptr: u64, path_len: u64) -> i64 {
    if path_ptr == 0 {
        return EFAULT;
    }
    
    let path_bytes = unsafe {
        core::slice::from_raw_parts(path_ptr as *const u8, path_len as usize)
    };
    
    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => return EINVAL,
    };
    
    let mut vfs = VFS.lock();
    match vfs.create_dir(path) {
        Ok(()) => 0,
        Err(crate::fs::vfs::VfsError::AlreadyExists) => EEXIST,
        Err(crate::fs::vfs::VfsError::NotFound) => ENOENT,
        Err(_) => EINVAL,
    }
}

/// sys_stat — get file statistics.
pub fn sys_stat(path_ptr: u64, path_len: u64, stat_ptr: u64) -> i64 {
    if path_ptr == 0 || stat_ptr == 0 {
        return EFAULT;
    }
    
    let path_bytes = unsafe {
        core::slice::from_raw_parts(path_ptr as *const u8, path_len as usize)
    };
    
    let path = match core::str::from_utf8(path_bytes) {
        Ok(s) => s,
        Err(_) => return EINVAL,
    };
    
    let vfs = VFS.lock();
    match vfs.stat(path) {
        Ok(stat) => {
            // Write stat info to user buffer
            let out = unsafe { &mut *(stat_ptr as *mut UserStat) };
            out.inode = stat.inode;
            out.size = stat.size;
            out.file_type = match stat.file_type {
                crate::fs::vfs::FileType::Regular => 1,
                crate::fs::vfs::FileType::Directory => 2,
                crate::fs::vfs::FileType::SymLink => 3,
                crate::fs::vfs::FileType::Device => 4,
                crate::fs::vfs::FileType::Pipe => 5,
            };
            0
        }
        Err(_) => ENOENT,
    }
}

/// User-space stat structure
#[repr(C)]
pub struct UserStat {
    pub inode: u64,
    pub size: u64,
    pub file_type: u64,
}
