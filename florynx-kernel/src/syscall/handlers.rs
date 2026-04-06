// =============================================================================
// Florynx Kernel — Syscall Handlers
// =============================================================================
// Implementation of individual syscall handler functions.
// =============================================================================

/// sys_write — write data to a file descriptor.
/// Currently supports fd=1 (stdout → VGA) and fd=2 (stderr → serial).
pub fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> i64 {
    // In a real kernel, we'd validate the user pointer and copy from userspace.
    // For now, this is a kernel-mode stub.
    let buf = unsafe {
        core::slice::from_raw_parts(buf_ptr as *const u8, len as usize)
    };

    match fd {
        1 => {
            // stdout → VGA
            if let Ok(s) = core::str::from_utf8(buf) {
                crate::print!("{}", s);
            }
            len as i64
        }
        2 => {
            // stderr → serial
            if let Ok(s) = core::str::from_utf8(buf) {
                crate::serial_print!("{}", s);
            }
            len as i64
        }
        _ => -1, // EBADF
    }
}

/// sys_exit — terminate the current process.
pub fn sys_exit(exit_code: u64) -> i64 {
    crate::serial_println!("[syscall] exit with code {}", exit_code);
    // In a full implementation, this would remove the current task from the scheduler.
    0
}

/// sys_yield — yield the CPU to the scheduler.
pub fn sys_yield() -> i64 {
    crate::serial_println!("[syscall] yield");
    // In a full implementation, this would trigger a context switch.
    0
}

/// sys_getpid — return the current process ID.
pub fn sys_getpid() -> i64 {
    // Stub: return 1 (init process)
    1
}

/// sys_sleep — sleep for a given number of ticks.
pub fn sys_sleep(ticks: u64) -> i64 {
    let start = crate::drivers::timer::pit::get_ticks();
    while crate::drivers::timer::pit::get_ticks() - start < ticks {
        x86_64::instructions::hlt();
    }
    0
}
