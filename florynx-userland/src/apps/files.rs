// =============================================================================
// Florynx Userland — File Manager (Dolphin-Style)
// =============================================================================
// KDE Dolphin-inspired file manager with sidebar, breadcrumb path, file grid.
// Uses VFS syscalls (SYS_OPEN, SYS_READ, SYS_STAT, SYS_MKDIR).
// =============================================================================

/// File manager application state.
pub struct FileManager {
    pub current_path: [u8; 256],
    pub path_len: usize,
    pub sidebar_width: usize,
    pub view_mode: ViewMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Icons,
    List,
    Details,
}

impl FileManager {
    pub fn new() -> Self {
        let mut path = [0u8; 256];
        path[0] = b'/';
        FileManager {
            current_path: path,
            path_len: 1,
            sidebar_width: 180,
            view_mode: ViewMode::Icons,
        }
    }
}
