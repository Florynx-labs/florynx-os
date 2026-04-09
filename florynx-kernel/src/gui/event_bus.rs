// =============================================================================
// Florynx Kernel — GUI Input Event Queue
// =============================================================================
// Decouples IRQ handlers from compositor logic.
// IRQs enqueue events; desktop drains and processes them in redraw loop.
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::gui::event::Key;

const QUEUE_SIZE: usize = 256;

#[derive(Clone, Copy)]
pub enum GuiInputEvent {
    MouseState { x: usize, y: usize, buttons: u8 },
    KeyPress { key: Key },
}

pub struct GuiInputQueue {
    events: [Option<GuiInputEvent>; QUEUE_SIZE],
    head: usize,
    tail: usize,
}

impl GuiInputQueue {
    const fn new() -> Self {
        Self {
            events: [None; QUEUE_SIZE],
            head: 0,
            tail: 0,
        }
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    fn next(idx: usize) -> usize {
        (idx + 1) % QUEUE_SIZE
    }

    fn push(&mut self, event: GuiInputEvent) {
        self.events[self.tail] = Some(event);
        self.tail = Self::next(self.tail);
        if self.tail == self.head {
            // Drop oldest on overflow to keep IRQ path non-blocking.
            self.head = Self::next(self.head);
            INPUT_DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn pop(&mut self) -> Option<GuiInputEvent> {
        if self.is_empty() {
            return None;
        }
        let ev = self.events[self.head];
        self.events[self.head] = None;
        self.head = Self::next(self.head);
        ev
    }
}

pub static INPUT_QUEUE: Mutex<GuiInputQueue> = Mutex::new(GuiInputQueue::new());
static INPUT_DROPPED: AtomicU64 = AtomicU64::new(0);

// ---------------------------------------------------------------------------
// Userland-facing GUI event queue (packed u64 events).
// ---------------------------------------------------------------------------

const USER_QUEUE_SIZE: usize = 256;

pub struct UserEventQueue {
    events: [u64; USER_QUEUE_SIZE],
    head: usize,
    tail: usize,
}

impl UserEventQueue {
    const fn new() -> Self {
        Self {
            events: [0; USER_QUEUE_SIZE],
            head: 0,
            tail: 0,
        }
    }

    fn next(idx: usize) -> usize {
        (idx + 1) % USER_QUEUE_SIZE
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    fn push(&mut self, event: u64) {
        self.events[self.tail] = event;
        self.tail = Self::next(self.tail);
        if self.tail == self.head {
            // Drop oldest on overflow.
            self.head = Self::next(self.head);
            USER_DROPPED.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn pop(&mut self) -> Option<u64> {
        if self.is_empty() {
            return None;
        }
        let ev = self.events[self.head];
        self.head = Self::next(self.head);
        Some(ev)
    }
}

pub static USER_EVENT_QUEUE: Mutex<UserEventQueue> = Mutex::new(UserEventQueue::new());
static USER_DROPPED: AtomicU64 = AtomicU64::new(0);

pub fn push_mouse_state(x: usize, y: usize, buttons: u8) {
    if let Some(mut q) = INPUT_QUEUE.try_lock() {
        q.push(GuiInputEvent::MouseState { x, y, buttons });
    } else {
        INPUT_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn push_key_press(key: Key) {
    if let Some(mut q) = INPUT_QUEUE.try_lock() {
        q.push(GuiInputEvent::KeyPress { key });
    } else {
        INPUT_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn pop_event() -> Option<GuiInputEvent> {
    let mut q = INPUT_QUEUE.lock();
    q.pop()
}

/// Event type 1: mouse state update.
pub fn push_user_mouse_event(win_id: u32, x: usize, y: usize, buttons: u8) {
    // [type:8][win_id:16][buttons:8][x:16][y:16]
    let packed = (1u64 << 56)
        | (((win_id as u64) & 0xFFFF) << 40)
        | ((buttons as u64) << 32)
        | (((x as u64) & 0xFFFF) << 16)
        | ((y as u64) & 0xFFFF);
    if let Some(mut q) = USER_EVENT_QUEUE.try_lock() {
        q.push(packed);
    } else {
        USER_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Event type 2: key press.
pub fn push_user_key_event(win_id: u32, code: u16) {
    // [type:8][win_id:16][code:16]
    let packed = (2u64 << 56)
        | (((win_id as u64) & 0xFFFF) << 40)
        | ((code as u64) << 24);
    if let Some(mut q) = USER_EVENT_QUEUE.try_lock() {
        q.push(packed);
    } else {
        USER_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Event type 3: window created.
pub fn push_user_window_created(win_id: u32) {
    // [type:8][win_id:16]
    let packed = (3u64 << 56) | (((win_id as u64) & 0xFFFF) << 40);
    if let Some(mut q) = USER_EVENT_QUEUE.try_lock() {
        q.push(packed);
    } else {
        USER_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

/// Event type 4: window destroyed.
pub fn push_user_window_destroyed(win_id: u32) {
    // [type:8][win_id:16]
    let packed = (4u64 << 56) | (((win_id as u64) & 0xFFFF) << 40);
    if let Some(mut q) = USER_EVENT_QUEUE.try_lock() {
        q.push(packed);
    } else {
        USER_DROPPED.fetch_add(1, Ordering::Relaxed);
    }
}

pub fn pop_user_event() -> Option<u64> {
    let mut q = USER_EVENT_QUEUE.lock();
    q.pop()
}

#[derive(Clone, Copy, Debug)]
pub struct QueueDropTelemetry {
    pub gui_input_dropped: u64,
    pub user_event_dropped: u64,
}

pub fn drop_telemetry() -> QueueDropTelemetry {
    QueueDropTelemetry {
        gui_input_dropped: INPUT_DROPPED.load(Ordering::Relaxed),
        user_event_dropped: USER_DROPPED.load(Ordering::Relaxed),
    }
}

