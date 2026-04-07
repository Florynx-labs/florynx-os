// =============================================================================
// Florynx Kernel — Security: Capability System
// =============================================================================
// Bitflag-based capability tokens for fine-grained access control.
// Every task carries a CapabilitySet. Syscalls check required caps
// before performing privileged operations.
// =============================================================================

/// Capability flags — each bit grants a specific privilege.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability(pub u64);

impl Capability {
    // ---- Filesystem ----
    pub const FS_READ:      Capability = Capability(1 << 0);
    pub const FS_WRITE:     Capability = Capability(1 << 1);
    pub const FS_CREATE:    Capability = Capability(1 << 2);
    pub const FS_DELETE:    Capability = Capability(1 << 3);

    // ---- Process ----
    pub const PROC_SPAWN:   Capability = Capability(1 << 4);
    pub const PROC_KILL:    Capability = Capability(1 << 5);

    // ---- GUI ----
    pub const GUI_WINDOW:   Capability = Capability(1 << 6);
    pub const GUI_INPUT:    Capability = Capability(1 << 7);

    // ---- Network (future) ----
    pub const NET_LISTEN:   Capability = Capability(1 << 8);
    pub const NET_CONNECT:  Capability = Capability(1 << 9);

    // ---- Hardware ----
    pub const HW_IO:        Capability = Capability(1 << 10);
    pub const HW_IRQ:       Capability = Capability(1 << 11);

    // ---- IPC ----
    pub const IPC_SEND:     Capability = Capability(1 << 12);
    pub const IPC_RECV:     Capability = Capability(1 << 13);

    // ---- Clock ----
    pub const CLOCK_READ:   Capability = Capability(1 << 14);
    pub const CLOCK_SET:    Capability = Capability(1 << 15);

    // ---- Memory ----
    pub const MEM_MAP:      Capability = Capability(1 << 16);
    pub const MEM_ALLOC:    Capability = Capability(1 << 17);

    // ---- Admin ----
    pub const ADMIN:        Capability = Capability(1 << 63);

    pub const NONE: Capability = Capability(0);
}

/// A set of capabilities held by a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CapabilitySet {
    bits: u64,
}

impl CapabilitySet {
    /// Empty capability set — no permissions.
    pub const fn empty() -> Self {
        CapabilitySet { bits: 0 }
    }

    /// Full capabilities — kernel mode (all bits set).
    pub const fn kernel() -> Self {
        CapabilitySet { bits: u64::MAX }
    }

    /// Default user capabilities (basic FS + GUI + IPC + clock).
    pub const fn user_default() -> Self {
        CapabilitySet {
            bits: Capability::FS_READ.0
                | Capability::FS_WRITE.0
                | Capability::GUI_WINDOW.0
                | Capability::GUI_INPUT.0
                | Capability::IPC_SEND.0
                | Capability::IPC_RECV.0
                | Capability::CLOCK_READ.0,
        }
    }

    /// Sandboxed capabilities — minimal (read-only FS + GUI input).
    pub const fn sandboxed() -> Self {
        CapabilitySet {
            bits: Capability::FS_READ.0
                | Capability::GUI_INPUT.0
                | Capability::CLOCK_READ.0,
        }
    }

    /// Check if this set contains the given capability.
    pub const fn has(&self, cap: Capability) -> bool {
        (self.bits & cap.0) == cap.0
    }

    /// Grant a capability.
    pub fn grant(&mut self, cap: Capability) {
        self.bits |= cap.0;
    }

    /// Revoke a capability.
    pub fn revoke(&mut self, cap: Capability) {
        self.bits &= !cap.0;
    }

    /// Merge another set into this one (union).
    pub fn merge(&mut self, other: CapabilitySet) {
        self.bits |= other.bits;
    }

    /// Intersect with another set.
    pub fn intersect(&self, other: CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            bits: self.bits & other.bits,
        }
    }

    /// Get the raw bits.
    pub const fn bits(&self) -> u64 {
        self.bits
    }
}

// =============================================================================
// Capability Checking
// =============================================================================

/// Error returned when a capability check fails.
#[derive(Debug, Clone, Copy)]
pub struct CapError {
    pub required: Capability,
    pub task_caps: u64,
}

/// Check that the given capability set contains the required cap.
/// Returns Ok(()) if granted, Err(CapError) if denied.
pub fn check_capability(caps: &CapabilitySet, required: Capability) -> Result<(), CapError> {
    if caps.has(required) {
        Ok(())
    } else {
        Err(CapError {
            required,
            task_caps: caps.bits(),
        })
    }
}

// =============================================================================
// Legacy compat — CapabilityTable (used by process.rs)
// =============================================================================

/// Legacy capability table — wraps CapabilitySet.
pub struct CapabilityTable {
    pub caps: CapabilitySet,
}

impl CapabilityTable {
    pub fn new() -> Self {
        CapabilityTable {
            caps: CapabilitySet::kernel(), // Default to kernel caps
        }
    }

    pub fn with_caps(caps: CapabilitySet) -> Self {
        CapabilityTable { caps }
    }

    pub fn has(&self, cap: Capability) -> bool {
        self.caps.has(cap)
    }

    pub fn check(&self, required: Capability) -> Result<(), CapError> {
        check_capability(&self.caps, required)
    }

    pub fn grant(&mut self, cap: Capability) {
        self.caps.grant(cap);
    }

    pub fn revoke(&mut self, cap: Capability) {
        self.caps.revoke(cap);
    }
}
