// =============================================================================
// Florynx Kernel — Syscall Number Table
// =============================================================================
// Defines the syscall numbers for the Florynx kernel ABI.
// =============================================================================

/// Write data to a file descriptor.
pub const SYS_WRITE: u64 = 1;

/// Terminate the current process.
pub const SYS_EXIT: u64 = 60;

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
