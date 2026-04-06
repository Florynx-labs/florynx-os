// =============================================================================
// Florynx Kernel — Security: Capability System
// =============================================================================

use alloc::vec::Vec;

/// A capability token granting specific permissions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Capability {
    pub id: u64,
    pub permissions: Permissions,
}

/// Permission bit flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Permissions(pub u32);

impl Permissions {
    pub const NONE: Permissions = Permissions(0);
    pub const READ: Permissions = Permissions(1 << 0);
    pub const WRITE: Permissions = Permissions(1 << 1);
    pub const EXECUTE: Permissions = Permissions(1 << 2);
    pub const ADMIN: Permissions = Permissions(1 << 3);

    pub fn has(self, perm: Permissions) -> bool {
        (self.0 & perm.0) == perm.0
    }

    pub fn grant(self, perm: Permissions) -> Permissions {
        Permissions(self.0 | perm.0)
    }
}

/// Capability table for a process.
pub struct CapabilityTable {
    capabilities: Vec<Capability>,
}

impl CapabilityTable {
    pub fn new() -> Self {
        CapabilityTable {
            capabilities: Vec::new(),
        }
    }

    pub fn add(&mut self, cap: Capability) {
        self.capabilities.push(cap);
    }

    pub fn has_capability(&self, id: u64) -> bool {
        self.capabilities.iter().any(|c| c.id == id)
    }

    pub fn check(&self, id: u64, required: Permissions) -> bool {
        self.capabilities
            .iter()
            .any(|c| c.id == id && c.permissions.has(required))
    }
}
