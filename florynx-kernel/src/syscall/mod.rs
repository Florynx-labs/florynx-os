// =============================================================================
// Florynx Kernel — Syscall Interface
// =============================================================================
// Production-level syscall dispatcher.
// Routes syscall numbers to handler functions.
// Convention: RAX=syscall_num, RDI=arg1, RSI=arg2, RDX=arg3
// Return value in RAX.
// =============================================================================

pub mod table;
pub mod handlers;

// POSIX error
const ENOSYS: i64 = -38;

/// Dispatch a syscall by number.
/// Arguments follow Linux x86_64 ABI: RDI, RSI, RDX
pub fn dispatch(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    match syscall_num {
        table::SYS_READ   => handlers::sys_read(arg1, arg2, arg3),
        table::SYS_WRITE  => handlers::sys_write(arg1, arg2, arg3),
        table::SYS_OPEN   => handlers::sys_open(arg1, arg2, arg3),
        table::SYS_CLOSE  => handlers::sys_close(arg1),
        table::SYS_STAT   => handlers::sys_stat(arg1, arg2, arg3),
        table::SYS_SEEK   => handlers::sys_seek(arg1, arg2, arg3),
        table::SYS_YIELD  => handlers::sys_yield(),
        table::SYS_SLEEP  => handlers::sys_sleep(arg1),
        table::SYS_GETPID => handlers::sys_getpid(),
        table::SYS_EXIT   => handlers::sys_exit(arg1),
        table::SYS_MKDIR  => handlers::sys_mkdir(arg1, arg2),
        _ => {
            crate::serial_println!("[syscall] unknown syscall: {}", syscall_num);
            ENOSYS
        }
    }
}

/// Initialize syscall interface
pub fn init() {
    crate::serial_println!("[syscall] interface initialized (11 syscalls registered)");
    crate::serial_println!("[syscall]   SYS_READ={}, SYS_WRITE={}, SYS_OPEN={}, SYS_CLOSE={}", 
        table::SYS_READ, table::SYS_WRITE, table::SYS_OPEN, table::SYS_CLOSE);
    crate::serial_println!("[syscall]   SYS_STAT={}, SYS_SEEK={}, SYS_YIELD={}, SYS_SLEEP={}", 
        table::SYS_STAT, table::SYS_SEEK, table::SYS_YIELD, table::SYS_SLEEP);
    crate::serial_println!("[syscall]   SYS_GETPID={}, SYS_EXIT={}, SYS_MKDIR={}", 
        table::SYS_GETPID, table::SYS_EXIT, table::SYS_MKDIR);
}
