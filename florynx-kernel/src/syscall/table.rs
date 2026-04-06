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

/// Open a file (stub).
pub const SYS_OPEN: u64 = 2;

/// Close a file (stub).
pub const SYS_CLOSE: u64 = 3;

/// Read from a file (stub).
pub const SYS_READ: u64 = 0;

/// Memory map (stub).
pub const SYS_MMAP: u64 = 9;
