// =============================================================================
// Florynx Userland — Taskbar (KDE Plasma-Style)
// =============================================================================
// Shows running windows as buttons in the panel center zone.
// Active window is highlighted with the accent color indicator.
// =============================================================================

use alloc::string::String;

const MAX_TASKS: usize = 16;

/// A single task entry on the taskbar.
#[derive(Clone)]
pub struct TaskEntry {
    pub win_id: u32,
    pub title: String,
    pub active: bool,
    pub icon_idx: usize,
}

/// Taskbar state.
pub struct Taskbar {
    entries: [Option<TaskEntry>; MAX_TASKS],
    count: usize,
}

impl Taskbar {
    pub const fn new() -> Self {
        const NONE: Option<TaskEntry> = None;
        Taskbar {
            entries: [NONE; MAX_TASKS],
            count: 0,
        }
    }

    pub fn add(&mut self, win_id: u32, title: &str, icon_idx: usize) {
        if self.count >= MAX_TASKS { return; }
        for i in 0..MAX_TASKS {
            if self.entries[i].is_none() {
                self.entries[i] = Some(TaskEntry {
                    win_id,
                    title: String::from(title),
                    active: false,
                    icon_idx,
                });
                self.count += 1;
                return;
            }
        }
    }

    pub fn remove(&mut self, win_id: u32) {
        for i in 0..MAX_TASKS {
            if let Some(ref e) = self.entries[i] {
                if e.win_id == win_id {
                    self.entries[i] = None;
                    self.count = self.count.saturating_sub(1);
                    return;
                }
            }
        }
    }

    pub fn set_active(&mut self, win_id: u32) {
        for i in 0..MAX_TASKS {
            if let Some(ref mut e) = self.entries[i] {
                e.active = e.win_id == win_id;
            }
        }
    }

    pub fn entries(&self) -> impl Iterator<Item = &TaskEntry> {
        self.entries.iter().filter_map(|e| e.as_ref())
    }

    pub fn count(&self) -> usize {
        self.count
    }

    /// Get entry at click position within the taskbar zone.
    pub fn entry_at(&self, offset_x: usize, zone_w: usize) -> Option<u32> {
        if self.count == 0 { return None; }
        let btn_w = (zone_w / self.count).min(180);
        let idx = offset_x / btn_w;
        let mut n = 0;
        for i in 0..MAX_TASKS {
            if let Some(ref e) = self.entries[i] {
                if n == idx { return Some(e.win_id); }
                n += 1;
            }
        }
        None
    }
}
