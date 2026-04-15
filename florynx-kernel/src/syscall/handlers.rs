// =============================================================================
// Florynx Kernel — Syscall Handlers
// =============================================================================
// Production-level syscall handler implementations.
// Integrates with VFS, scheduler, and I/O subsystems.
// =============================================================================

use crate::fs::vfs::{VFS, OpenFlags};
use crate::process::scheduler;
use crate::syscall::usermem;
use crate::gui::desktop;
use florynx_shared::syscall_abi::{AbiHeader, AbiInfoV1, KernelTelemetryV1, UserStatV1, ABI_V1};

// Error codes (POSIX-compatible)
const EBADF: i64 = -9;
const EFAULT: i64 = -14;
const EINVAL: i64 = -22;
const ENOENT: i64 = -2;
const EACCES: i64 = -13;
const ECHILD: i64 = -10;
const EAGAIN: i64 = -11;
const EISDIR: i64 = -21;
const EEXIST: i64 = -17;
const ESRCH: i64 = -3;
const ENOSYS: i64 = -38;

/// sys_write — write data to a file descriptor.
/// fd=0: stdin (invalid for write)
/// fd=1: stdout → serial + VGA
/// fd=2: stderr → serial
/// fd>=3: VFS file descriptor
pub fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> i64 {
    let buf = match usermem::copy_from_user(buf_ptr, len) {
        Ok(s) => s,
        Err(e) => return e,
    };

    match fd {
        0 => EBADF, // stdin not writable
        1 => {
            // stdout → serial + VGA
            if let Ok(s) = core::str::from_utf8(&buf) {
                crate::serial_print!("{}", s);
                crate::print!("{}", s);
            }
            len as i64
        }
        2 => {
            // stderr → serial only
            if let Ok(s) = core::str::from_utf8(&buf) {
                crate::serial_print!("{}", s);
            }
            len as i64
        }
        fd => {
            // VFS file descriptor
            let mut vfs = VFS.lock();
            match vfs.write(fd as usize, &buf) {
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
    // SECURITY: Clamp read_len to a sane max (64KB) to prevent userland OOM DOS attacks
    let read_len = core::cmp::min(len, 64 * 1024);
    let mut buf = alloc::vec![0u8; read_len as usize];

    match fd {
        0 => {
            // stdin — currently no keyboard buffer, return 0
            0
        }
        1 | 2 => EBADF, // stdout/stderr not readable
        fd => {
            // VFS file descriptor
            let mut vfs = VFS.lock();
            match vfs.read(fd as usize, &mut buf) {
                Ok(n) => {
                    if usermem::copy_to_user(buf_ptr, &buf[..n]).is_err() {
                        return EFAULT;
                    }
                    n as i64
                }
                Err(_) => EBADF,
            }
        }
    }
}

/// sys_open — open a file by path.
/// flags: 0=read, 1=write, 2=read+write, 4=create
pub fn sys_open(path_ptr: u64, path_len: u64, flags: u64) -> i64 {
    let path_bytes = match usermem::copy_from_user(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let path = match core::str::from_utf8(&path_bytes) {
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
    scheduler::exit_with_code(exit_code);
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
    scheduler::sleep_current(ticks);
    0
}

/// sys_wait — reap any zombie child.
/// arg1=out_ptr where [child_id:u64, exit_code:u64] is written.
/// arg2=flags (bit0: WNOHANG)
pub fn sys_wait(out_ptr: u64, flags: u64, _arg3: u64) -> i64 {
    const WNOHANG: u64 = 1;
    match scheduler::wait_any_child() {
        Some((child, code)) => {
            let mut out = [0u8; 16];
            out[..8].copy_from_slice(&child.0.to_ne_bytes());
            out[8..].copy_from_slice(&code.to_ne_bytes());
            match usermem::copy_to_user(out_ptr, &out) {
                Ok(()) => 0,
                Err(e) => e,
            }
        }
        None => {
            if !scheduler::has_any_child() {
                ECHILD
            } else if (flags & WNOHANG) != 0 {
                EAGAIN
            } else {
                // Simple blocking behavior: sleep one tick and let caller retry.
                scheduler::sleep_current(1);
                EAGAIN
            }
        }
    }
}

/// sys_kill — send a signal to a task.
/// arg1=pid, arg2=signum (SIGKILL=9, SIGTERM=15, etc.)
pub fn sys_kill(pid: u64, signum: u64, _arg3: u64) -> i64 {
    use crate::process::signal;
    if signum > signal::SIGMAX as u64 {
        return EINVAL;
    }
    if signum == 0 {
        // Signal 0: existence check only.
        return if scheduler::kill(crate::process::task::TaskId(pid), 0) { 0 } else { ESRCH };
    }
    if scheduler::send_signal(crate::process::task::TaskId(pid), signum as u32) {
        0
    } else {
        ESRCH
    }
}

/// sys_waitpid — wait for a specific child task.
/// arg1=pid, arg2=out_ptr (&u64 exit_code), arg3=flags (bit0=WNOHANG)
pub fn sys_waitpid(pid: u64, out_ptr: u64, flags: u64) -> i64 {
    const WNOHANG: u64 = 1;
    let child_id = crate::process::task::TaskId(pid);
    loop {
        match scheduler::wait_child_pid(child_id) {
            Some(code) => {
                if out_ptr != 0 {
                    let bytes = code.to_ne_bytes();
                    if let Err(e) = usermem::copy_to_user(out_ptr, &bytes) {
                        return e;
                    }
                }
                return pid as i64;
            }
            None => {
                if (flags & WNOHANG) != 0 {
                    return EAGAIN;
                }
                // Child exists but not yet dead — yield and retry.
                if !scheduler::has_any_child() {
                    return ECHILD;
                }
                scheduler::sleep_current(1);
            }
        }
    }
}

/// SYS_ABI_INFO
/// arg1=out_ptr, arg2=out_len, arg3=reserved
pub fn sys_abi_info(out_ptr: u64, out_len: u64, _arg3: u64) -> i64 {
    if out_len < core::mem::size_of::<AbiInfoV1>() as u64 {
        return EINVAL;
    }
    
    // SECURITY: Read and validate the caller's struct header
    let hdr_bytes = match usermem::copy_from_user(out_ptr, 4) {
        Ok(b) => b,
        Err(e) => return e,
    };
    let supplied_size = u16::from_ne_bytes([hdr_bytes[0], hdr_bytes[1]]);
    let supplied_version = u16::from_ne_bytes([hdr_bytes[2], hdr_bytes[3]]);
    
    if supplied_version != ABI_V1 || supplied_size < core::mem::size_of::<AbiInfoV1>() as u16 {
        return EINVAL;
    }

    let info = AbiInfoV1 {
        hdr: AbiHeader {
            size: core::mem::size_of::<AbiInfoV1>() as u16,
            version: ABI_V1,
        },
        abi_major: 1,
        abi_minor: 0,
        user_stat_size: core::mem::size_of::<UserStatV1>() as u32,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (&info as *const AbiInfoV1).cast::<u8>(),
            core::mem::size_of::<AbiInfoV1>(),
        )
    };
    match usermem::copy_to_user(out_ptr, bytes) {
        Ok(()) => 0,
        Err(e) => e,
    }
}

/// SYS_DEBUG_TELEMETRY
/// arg1=out_ptr, arg2=out_len, arg3=reserved
pub fn sys_debug_telemetry(out_ptr: u64, out_len: u64, _arg3: u64) -> i64 {
    if out_len < core::mem::size_of::<KernelTelemetryV1>() as u64 {
        return EINVAL;
    }
    
    // SECURITY: Read and validate the caller's struct header
    let hdr_bytes = match usermem::copy_from_user(out_ptr, 4) {
        Ok(b) => b,
        Err(e) => return e,
    };
    let supplied_version = u16::from_ne_bytes([hdr_bytes[2], hdr_bytes[3]]);
    if supplied_version != ABI_V1 {
        return EINVAL;
    }

    let fault = crate::arch::x86_64::idt::fault_telemetry();
    let panic = crate::core_kernel::panic::panic_telemetry();
    let out = KernelTelemetryV1 {
        hdr: AbiHeader {
            size: core::mem::size_of::<KernelTelemetryV1>() as u16,
            version: ABI_V1,
        },
        page_fault_total: fault.page_fault_total,
        page_fault_user: fault.page_fault_user,
        page_fault_kernel: fault.page_fault_kernel,
        panic_count: panic.panic_count,
    };
    let bytes = unsafe {
        core::slice::from_raw_parts(
            (&out as *const KernelTelemetryV1).cast::<u8>(),
            core::mem::size_of::<KernelTelemetryV1>(),
        )
    };
    match usermem::copy_to_user(out_ptr, bytes) {
        Ok(()) => 0,
        Err(e) => e,
    }
}

/// sys_mkdir — create a directory.
pub fn sys_mkdir(path_ptr: u64, path_len: u64) -> i64 {
    let path_bytes = match usermem::copy_from_user(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    let path = match core::str::from_utf8(&path_bytes) {
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
    let path_bytes = match usermem::copy_from_user(path_ptr, path_len) {
        Ok(s) => s,
        Err(e) => return e,
    };
    
    // SECURITY: Validate stat_ptr header
    if stat_ptr != 0 {
        let hdr_bytes = match usermem::copy_from_user(stat_ptr, 4) {
            Ok(b) => b,
            Err(_) => return EINVAL,
        };
        let supplied_version = u16::from_ne_bytes([hdr_bytes[2], hdr_bytes[3]]);
        if supplied_version != ABI_V1 {
            return EINVAL; // ABI mismatch
        }
    }

    let path = match core::str::from_utf8(&path_bytes) {
        Ok(s) => s,
        Err(_) => return EINVAL,
    };
    
    let vfs = VFS.lock();
    match vfs.stat(path) {
        Ok(stat) => {
            let out = UserStatV1 {
                hdr: AbiHeader {
                    size: core::mem::size_of::<UserStatV1>() as u16,
                    version: ABI_V1,
                },
                inode: stat.inode,
                size: stat.size,
                file_type: match stat.file_type {
                    crate::fs::vfs::FileType::Regular => 1,
                    crate::fs::vfs::FileType::Directory => 2,
                    crate::fs::vfs::FileType::SymLink => 3,
                    crate::fs::vfs::FileType::Device => 4,
                    crate::fs::vfs::FileType::Pipe => 5,
                },
            };
            let out_bytes = unsafe {
                core::slice::from_raw_parts(
                    (&out as *const UserStatV1).cast::<u8>(),
                    core::mem::size_of::<UserStatV1>(),
                )
            };
            if let Err(e) = usermem::copy_to_user(stat_ptr, out_bytes) {
                return e;
            }
            0
        }
        Err(_) => ENOENT,
    }
}

/// SYS_GUI_CREATE_WINDOW
/// arg1=x, arg2=y, arg3=packed_wh (upper32=w, lower32=h)
pub fn sys_gui_create_window(x: u64, y: u64, packed_wh: u64) -> i64 {
    let w = ((packed_wh >> 32) & 0xFFFF_FFFF) as usize;
    let h = (packed_wh & 0xFFFF_FFFF) as usize;
    match desktop::create_user_window(x as usize, y as usize, w, h, "Userland App") {
        Some(id) => id as i64,
        None => EINVAL,
    }
}

/// SYS_GUI_DRAW_RECT
/// Minimal v1: accept call and request redraw.
pub fn sys_gui_draw_rect(_win_id: u64, _packed_xy: u64, _packed_wh_color: u64) -> i64 {
    let win_id = _win_id as usize;
    let x = ((_packed_xy >> 32) & 0xFFFF_FFFF) as usize;
    let y = (_packed_xy & 0xFFFF_FFFF) as usize;
    let w = ((_packed_wh_color >> 48) & 0xFFFF) as usize;
    let h = ((_packed_wh_color >> 32) & 0xFFFF) as usize;
    let rgb = (_packed_wh_color & 0xFFFF_FFFF) as u32;
    if desktop::set_window_rect(win_id, x, y, w.max(1), h.max(1), rgb) {
        desktop::request_redraw();
        0
    } else {
        EINVAL
    }
}

/// SYS_GUI_DRAW_TEXT
/// arg1=win_id, arg2=text_ptr, arg3=text_len
pub fn sys_gui_draw_text(win_id: u64, text_ptr: u64, text_len: u64) -> i64 {
    let text_bytes = match usermem::copy_from_user(text_ptr, text_len) {
        Ok(s) => s,
        Err(e) => return e,
    };
    let text = match core::str::from_utf8(&text_bytes) {
        Ok(s) => s,
        Err(_) => return EINVAL,
    };
    if desktop::set_window_text(win_id as usize, text) {
        desktop::request_redraw();
        0
    } else {
        EINVAL
    }
}

/// SYS_GUI_POLL_EVENT
/// Minimal v1: no event delivery yet.
pub fn sys_gui_poll_event(_event_ptr: u64, _arg2: u64, _arg3: u64) -> i64 {
    if let Some(ev) = crate::gui::event_bus::pop_user_event() {
        if let Err(e) = usermem::copy_to_user(_event_ptr, &ev.to_ne_bytes()) {
            return e;
        }
        1
    } else {
        0
    }
}

/// SYS_GUI_INVALIDATE
pub fn sys_gui_invalidate(_win_id: u64, _arg2: u64, _arg3: u64) -> i64 {
    desktop::request_redraw();
    0
}

/// SYS_GUI_FOCUS_WINDOW
pub fn sys_gui_focus_window(_win_id: u64, _arg2: u64, _arg3: u64) -> i64 {
    if desktop::focus_window(_win_id as usize) {
        0
    } else {
        EINVAL
    }
}

/// SYS_GUI_DESTROY_WINDOW
pub fn sys_gui_destroy_window(_win_id: u64, _arg2: u64, _arg3: u64) -> i64 {
    if desktop::destroy_window(_win_id as usize) {
        0
    } else {
        EINVAL
    }
}

/// SYS_GUI_SET_WALLPAPER (not implemented yet)
pub fn sys_gui_set_wallpaper(_path_ptr: u64, _path_len: u64, _arg3: u64) -> i64 {
    ENOSYS
}

/// SYS_CLOCK_GETTIME
/// clockid=0 → CLOCK_REALTIME  (RTC-based wall clock, seconds since epoch)
/// clockid=1 → CLOCK_MONOTONIC (PIT uptime, nanosecond precision via ticks)
/// Writes two u64 values to buf_ptr: [seconds, nanoseconds]
pub fn sys_clock_gettime(clockid: u64, buf_ptr: u64, _arg3: u64) -> i64 {
    let (secs, nanos): (u64, u64) = match clockid {
        0 => {
            let unix = crate::time::rtc::now_unix();
            let ticks = crate::drivers::timer::pit::get_ticks();
            let sub_sec_nanos = ((ticks % 200) * 1_000_000_000) / 200;
            (unix, sub_sec_nanos)
        }
        1 => {
            let ticks = crate::drivers::timer::pit::get_ticks();
            let secs = ticks / 200;
            let nanos = ((ticks % 200) * 1_000_000_000) / 200;
            (secs, nanos)
        }
        _ => return EINVAL,
    };

    let bytes_s = secs.to_ne_bytes();
    let bytes_n = nanos.to_ne_bytes();
    if let Err(e) = usermem::copy_to_user(buf_ptr, &bytes_s) { return e; }
    if let Err(e) = usermem::copy_to_user(buf_ptr + 8, &bytes_n) { return e; }
    0
}

/// sys_fork — clone the current process.
/// Returns child TaskId (> 0) in parent; child sees 0 in rax via initial_rax.
pub fn sys_fork() -> i64 {
    crate::process::fork::sys_fork()
}

/// sys_execve — replace current process image with a binary from the VFS.
/// arg1 = user pointer to null-terminated path string.
/// arg2/arg3 = argv/envp pointers (ignored in this implementation).
pub fn sys_execve(path_ptr: u64, argv: u64, envp: u64) -> i64 {
    crate::process::exec::sys_execve(path_ptr, argv, envp)
}
