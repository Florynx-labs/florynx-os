// =============================================================================
// Florynx Userland — Notification Daemon
// =============================================================================
// KDE-style notification popups (top-right corner).
// =============================================================================

use alloc::string::String;

const MAX_NOTIFICATIONS: usize = 8;

#[derive(Clone)]
pub struct Notification {
    pub title: String,
    pub body: String,
    pub icon_idx: usize,
    pub timeout_ms: u32,
    pub age_ms: u32,
}

pub struct NotificationDaemon {
    notifications: [Option<Notification>; MAX_NOTIFICATIONS],
    count: usize,
}

impl NotificationDaemon {
    pub const fn new() -> Self {
        const NONE: Option<Notification> = None;
        NotificationDaemon {
            notifications: [NONE; MAX_NOTIFICATIONS],
            count: 0,
        }
    }

    pub fn push(&mut self, title: &str, body: &str, icon_idx: usize, timeout_ms: u32) {
        if self.count >= MAX_NOTIFICATIONS { return; }
        for i in 0..MAX_NOTIFICATIONS {
            if self.notifications[i].is_none() {
                self.notifications[i] = Some(Notification {
                    title: String::from(title),
                    body: String::from(body),
                    icon_idx,
                    timeout_ms,
                    age_ms: 0,
                });
                self.count += 1;
                return;
            }
        }
    }

    pub fn tick(&mut self, delta_ms: u32) {
        for i in 0..MAX_NOTIFICATIONS {
            if let Some(ref mut n) = self.notifications[i] {
                n.age_ms += delta_ms;
                if n.age_ms >= n.timeout_ms {
                    self.notifications[i] = None;
                    self.count = self.count.saturating_sub(1);
                }
            }
        }
    }

    pub fn active(&self) -> impl Iterator<Item = &Notification> {
        self.notifications.iter().filter_map(|n| n.as_ref())
    }
}
