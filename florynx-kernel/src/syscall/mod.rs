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
pub mod usermem;

use crate::process::scheduler;
use crate::security::audit;
use crate::security::capability::{check_capability, Capability};

// POSIX error
const ENOSYS: i64 = -38;
const EPERM: i64 = -1;

#[inline]
fn required_capability(syscall_num: u64) -> Option<Capability> {
    match syscall_num {
        table::SYS_READ | table::SYS_STAT => Some(Capability::FS_READ),
        table::SYS_WRITE => Some(Capability::FS_WRITE),
        table::SYS_OPEN => Some(Capability::FS_READ),
        table::SYS_CLOSE | table::SYS_SEEK => Some(Capability::FS_READ),
        table::SYS_MKDIR => Some(Capability::FS_CREATE),
        table::SYS_GUI_CREATE_WINDOW
        | table::SYS_GUI_DESTROY_WINDOW
        | table::SYS_GUI_DRAW_RECT
        | table::SYS_GUI_DRAW_TEXT
        | table::SYS_GUI_POLL_EVENT
        | table::SYS_GUI_SET_WALLPAPER
        | table::SYS_GUI_INVALIDATE
        | table::SYS_GUI_BLIT_BUFFER
        | table::SYS_GUI_FOCUS_WINDOW => Some(Capability::GUI_WINDOW),
        table::SYS_ABI_INFO
        | table::SYS_YIELD
        | table::SYS_SLEEP
        | table::SYS_GETPID
        | table::SYS_EXIT
        | table::SYS_WAIT
        | table::SYS_KILL
        | table::SYS_WAITPID
        | table::SYS_DEBUG_TELEMETRY
        | table::SYS_CLOCK_GETTIME
        | table::SYS_FORK
        | table::SYS_EXECVE => None,
        _ => None,
    }
}

/// Dispatch a syscall by number.
/// Arguments follow Linux x86_64 ABI: RDI, RSI, RDX
pub fn dispatch(syscall_num: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    if let Some(required) = required_capability(syscall_num) {
        if let Some(caps) = scheduler::current_task_capabilities() {
            if check_capability(&caps, required).is_err() {
                let task_id = scheduler::current_task_id().map(|t| t.0).unwrap_or(0);
                audit::log_cap_denied(task_id, required.0);
                return EPERM;
            }
        }
    }

    let result = match syscall_num {
        table::SYS_READ   => handlers::sys_read(arg1, arg2, arg3),
        table::SYS_WRITE  => handlers::sys_write(arg1, arg2, arg3),
        table::SYS_ABI_INFO => handlers::sys_abi_info(arg1, arg2, arg3),
        table::SYS_DEBUG_TELEMETRY => handlers::sys_debug_telemetry(arg1, arg2, arg3),
        table::SYS_OPEN   => handlers::sys_open(arg1, arg2, arg3),
        table::SYS_CLOSE  => handlers::sys_close(arg1),
        table::SYS_STAT   => handlers::sys_stat(arg1, arg2, arg3),
        table::SYS_SEEK   => handlers::sys_seek(arg1, arg2, arg3),
        table::SYS_YIELD  => handlers::sys_yield(),
        table::SYS_SLEEP  => handlers::sys_sleep(arg1),
        table::SYS_GETPID => handlers::sys_getpid(),
        table::SYS_EXIT   => handlers::sys_exit(arg1),
        table::SYS_WAIT    => handlers::sys_wait(arg1, arg2, arg3),
        table::SYS_KILL    => handlers::sys_kill(arg1, arg2, arg3),
        table::SYS_WAITPID => handlers::sys_waitpid(arg1, arg2, arg3),
        table::SYS_MKDIR  => handlers::sys_mkdir(arg1, arg2),
        table::SYS_GUI_CREATE_WINDOW => handlers::sys_gui_create_window(arg1, arg2, arg3),
        table::SYS_GUI_DESTROY_WINDOW => handlers::sys_gui_destroy_window(arg1, arg2, arg3),
        table::SYS_GUI_DRAW_RECT => handlers::sys_gui_draw_rect(arg1, arg2, arg3),
        table::SYS_GUI_DRAW_TEXT => handlers::sys_gui_draw_text(arg1, arg2, arg3),
        table::SYS_GUI_POLL_EVENT => handlers::sys_gui_poll_event(arg1, arg2, arg3),
        table::SYS_GUI_SET_WALLPAPER => handlers::sys_gui_set_wallpaper(arg1, arg2, arg3),
        table::SYS_GUI_INVALIDATE => handlers::sys_gui_invalidate(arg1, arg2, arg3),
        table::SYS_GUI_BLIT_BUFFER => handlers::sys_gui_blit_buffer(arg1, arg2, arg3),
        table::SYS_GUI_FOCUS_WINDOW => handlers::sys_gui_focus_window(arg1, arg2, arg3),
        table::SYS_CLOCK_GETTIME => handlers::sys_clock_gettime(arg1, arg2, arg3),
        table::SYS_FORK   => handlers::sys_fork(),
        table::SYS_EXECVE => handlers::sys_execve(arg1, arg2, arg3),
        _ => {
            crate::serial_println!("[syscall] unknown syscall: {}", syscall_num);
            ENOSYS
        }
    };
    // Deliver any signals that were queued while this syscall ran.
    scheduler::deliver_pending_signals();
    result
}


/// Initialize syscall interface
pub fn init() {
    crate::serial_println!("[syscall] interface initialized");
    crate::serial_println!("[syscall] ingress vector: int 0x80 (ring3 enabled)");
    crate::serial_println!("[syscall]   SYS_READ={}, SYS_WRITE={}, SYS_OPEN={}, SYS_CLOSE={}", 
        table::SYS_READ, table::SYS_WRITE, table::SYS_OPEN, table::SYS_CLOSE);
    crate::serial_println!("[syscall]   SYS_STAT={}, SYS_SEEK={}, SYS_YIELD={}, SYS_SLEEP={}", 
        table::SYS_STAT, table::SYS_SEEK, table::SYS_YIELD, table::SYS_SLEEP);
    crate::serial_println!("[syscall]   SYS_GETPID={}, SYS_EXIT={}, SYS_MKDIR={}", 
        table::SYS_GETPID, table::SYS_EXIT, table::SYS_MKDIR);
}
