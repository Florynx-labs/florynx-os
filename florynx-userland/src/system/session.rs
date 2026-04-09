// =============================================================================
// Florynx Userland — Session Manager
// =============================================================================
// Manages user session lifecycle: login, logout, lock screen.
// =============================================================================

/// Session state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    Active,
    Locked,
    LoggingOut,
}

pub struct SessionManager {
    pub state: SessionState,
    pub username: [u8; 32],
    pub username_len: usize,
}

impl SessionManager {
    pub fn new() -> Self {
        let mut name = [0u8; 32];
        let default = b"florynx";
        name[..default.len()].copy_from_slice(default);
        SessionManager {
            state: SessionState::Active,
            username: name,
            username_len: default.len(),
        }
    }
}
