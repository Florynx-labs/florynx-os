// =============================================================================
// Florynx Kernel — Syscall Interface
// =============================================================================
// Syscall dispatcher that routes syscall numbers to handler functions.
// =============================================================================

pub mod table;
pub mod handlers;

/// Dispatch a syscall by number.
pub fn dispatch(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    match syscall_num {
        table::SYS_WRITE => handlers::sys_write(arg1, arg2, arg3),
        table::SYS_EXIT => handlers::sys_exit(arg1),
        table::SYS_YIELD => handlers::sys_yield(),
        table::SYS_GETPID => handlers::sys_getpid(),
        table::SYS_SLEEP => handlers::sys_sleep(arg1),
        _ => {
            crate::serial_println!("[syscall] unknown syscall: {}", syscall_num);
            -1 // ENOSYS
        }
    }
}
