// =============================================================================
// Florynx Kernel — POSIX-subset Signal Definitions
// =============================================================================
// Supports the minimal signal set required for Phase 1.3:
//   SIGHUP  (1)  — terminal hangup
//   SIGINT  (2)  — keyboard interrupt (Ctrl+C)
//   SIGQUIT (3)  — quit
//   SIGKILL (9)  — unblockable kill
//   SIGTERM (15) — software termination
// Signals are stored as a u32 bitfield in each Task.
// =============================================================================

/// SIGHUP — hangup / terminal disconnect.
pub const SIGHUP: u32 = 1;
/// SIGINT — interactive interrupt (Ctrl+C).
pub const SIGINT: u32 = 2;
/// SIGQUIT — quit with core dump (treated as kill here).
pub const SIGQUIT: u32 = 3;
/// SIGKILL — unconditional kill, cannot be caught or ignored.
pub const SIGKILL: u32 = 9;
/// SIGTERM — polite termination request.
pub const SIGTERM: u32 = 15;

/// Maximum supported signal number.
pub const SIGMAX: u32 = 31;

/// Convert a signal number to the corresponding bit in the pending bitfield.
#[inline]
pub const fn sig_bit(signum: u32) -> u32 {
    if signum == 0 || signum > SIGMAX {
        0
    } else {
        1 << (signum - 1)
    }
}

/// Returns true if the given signal is fatal in the default disposition
/// (i.e. should terminate the task when delivered).
#[inline]
pub const fn is_fatal_default(signum: u32) -> bool {
    matches!(signum, SIGHUP | SIGINT | SIGQUIT | SIGKILL | SIGTERM)
}

/// Exit code used when a task is killed by a signal.
/// Follows Unix convention: 128 + signum.
#[inline]
pub const fn signal_exit_code(signum: u32) -> u64 {
    128 + signum as u64
}
