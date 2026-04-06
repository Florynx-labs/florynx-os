// =============================================================================
// Florynx Kernel — Security: Process Isolation
// =============================================================================

/// Isolation domain for a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsolationDomain {
    pub id: u64,
    pub level: IsolationLevel,
}

/// Level of isolation enforced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// No isolation (kernel mode).
    None,
    /// Basic user/kernel separation.
    UserKernel,
    /// Full process isolation with separate address spaces.
    Full,
    /// Sandboxed with restricted capabilities.
    Sandboxed,
}

impl IsolationDomain {
    pub fn kernel() -> Self {
        IsolationDomain {
            id: 0,
            level: IsolationLevel::None,
        }
    }

    pub fn user(id: u64) -> Self {
        IsolationDomain {
            id,
            level: IsolationLevel::UserKernel,
        }
    }

    pub fn sandboxed(id: u64) -> Self {
        IsolationDomain {
            id,
            level: IsolationLevel::Sandboxed,
        }
    }
}
