// =============================================================================
// Florynx Kernel — Syscall Number Table
// =============================================================================
// Defines the syscall numbers for the Florynx kernel ABI.
// =============================================================================

/// Write data to a file descriptor.
pub const SYS_WRITE: u64 = 1;
/// Query syscall ABI version and struct sizes.
pub const SYS_ABI_INFO: u64 = 0x00F0;
/// Query kernel debug telemetry counters.
pub const SYS_DEBUG_TELEMETRY: u64 = 0x00F1;

/// Terminate the current process.
pub const SYS_EXIT: u64 = 60;
/// Wait for child process state change.
pub const SYS_WAIT: u64 = 61;
/// Send signal/terminate process.
pub const SYS_KILL: u64 = 62;

/// Yield the CPU to the scheduler.
pub const SYS_YIELD: u64 = 24;

/// Get the current process ID.
pub const SYS_GETPID: u64 = 39;

/// Sleep for a duration (in ticks).
pub const SYS_SLEEP: u64 = 35;

/// Open a file.
pub const SYS_OPEN: u64 = 2;

/// Close a file.
pub const SYS_CLOSE: u64 = 3;

/// Read from a file.
pub const SYS_READ: u64 = 0;

/// Seek in a file.
pub const SYS_SEEK: u64 = 8;

/// Create a directory.
pub const SYS_MKDIR: u64 = 83;

/// Get file statistics.
pub const SYS_STAT: u64 = 4;

/// GUI extension syscalls.
pub const SYS_GUI_CREATE_WINDOW: u64 = 0x1000;
pub const SYS_GUI_DESTROY_WINDOW: u64 = 0x1001;
pub const SYS_GUI_DRAW_RECT: u64 = 0x1002;
pub const SYS_GUI_DRAW_TEXT: u64 = 0x1003;
pub const SYS_GUI_POLL_EVENT: u64 = 0x1004;
pub const SYS_GUI_SET_WALLPAPER: u64 = 0x1005;
pub const SYS_GUI_INVALIDATE: u64 = 0x1006;
pub const SYS_GUI_FOCUS_WINDOW: u64 = 0x1007;
