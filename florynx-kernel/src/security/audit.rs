// =============================================================================
// Florynx Kernel — Security: Audit Log
// =============================================================================
// Fixed-size ring buffer logging security events (denied capabilities,
// syscall violations, etc.). Oldest entries are overwritten.
// =============================================================================

use spin::Mutex;
use lazy_static::lazy_static;

const AUDIT_LOG_SIZE: usize = 256;

/// Type of audited event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditEventKind {
    CapabilityDenied,
    SyscallDenied,
    InvalidAccess,
    TaskTerminated,
    TaskSpawned,
}

/// A single audit log entry
#[derive(Debug, Clone, Copy)]
pub struct AuditEntry {
    pub tick: u64,
    pub task_id: u64,
    pub kind: AuditEventKind,
    pub detail: u64,
}

impl AuditEntry {
    const fn empty() -> Self {
        AuditEntry {
            tick: 0,
            task_id: 0,
            kind: AuditEventKind::TaskSpawned,
            detail: 0,
        }
    }
}

/// Fixed-size ring buffer for audit entries
pub struct AuditLog {
    entries: [AuditEntry; AUDIT_LOG_SIZE],
    head: usize,
    count: usize,
}

impl AuditLog {
    const fn new() -> Self {
        AuditLog {
            entries: [AuditEntry::empty(); AUDIT_LOG_SIZE],
            head: 0,
            count: 0,
        }
    }

    /// Log an event. Overwrites oldest if full.
    pub fn log(&mut self, task_id: u64, kind: AuditEventKind, detail: u64) {
        let tick = crate::drivers::timer::pit::get_ticks();
        self.entries[self.head] = AuditEntry {
            tick,
            task_id,
            kind,
            detail,
        };
        self.head = (self.head + 1) % AUDIT_LOG_SIZE;
        if self.count < AUDIT_LOG_SIZE {
            self.count += 1;
        }
    }

    /// Get total logged events
    pub fn count(&self) -> usize {
        self.count
    }

    /// Get the most recent N entries (newest first)
    pub fn recent(&self, n: usize) -> impl Iterator<Item = &AuditEntry> {
        let n = n.min(self.count);
        let start = if self.head >= n {
            self.head - n
        } else {
            AUDIT_LOG_SIZE - (n - self.head)
        };
        
        (0..n).map(move |i| {
            let idx = (start + i) % AUDIT_LOG_SIZE;
            &self.entries[idx]
        })
    }
}

lazy_static! {
    pub static ref AUDIT: Mutex<AuditLog> = Mutex::new(AuditLog::new());
}

/// Log a capability denial
pub fn log_cap_denied(task_id: u64, cap_bits: u64) {
    if let Some(mut audit) = AUDIT.try_lock() {
        audit.log(task_id, AuditEventKind::CapabilityDenied, cap_bits);
    }
    crate::serial_println!(
        "[audit] DENIED: task={} cap=0x{:x}",
        task_id, cap_bits
    );
}

/// Log a syscall denial
pub fn log_syscall_denied(task_id: u64, syscall_num: u64) {
    if let Some(mut audit) = AUDIT.try_lock() {
        audit.log(task_id, AuditEventKind::SyscallDenied, syscall_num);
    }
    crate::serial_println!(
        "[audit] DENIED syscall: task={} syscall={}",
        task_id, syscall_num
    );
}

/// Log task spawn
pub fn log_task_spawned(task_id: u64) {
    if let Some(mut audit) = AUDIT.try_lock() {
        audit.log(task_id, AuditEventKind::TaskSpawned, 0);
    }
}

/// Log task terminated
pub fn log_task_terminated(task_id: u64, exit_code: u64) {
    if let Some(mut audit) = AUDIT.try_lock() {
        audit.log(task_id, AuditEventKind::TaskTerminated, exit_code);
    }
}

/// Dump recent audit entries to serial
pub fn dump_recent(n: usize) {
    let audit = AUDIT.lock();
    crate::serial_println!("[audit] last {} events ({} total):", n, audit.count());
    for entry in audit.recent(n) {
        crate::serial_println!(
            "  [tick={}] task={} {:?} detail=0x{:x}",
            entry.tick, entry.task_id, entry.kind, entry.detail
        );
    }
}
