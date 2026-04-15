// =============================================================================
// Florynx Kernel — Driver Event Queue
// =============================================================================
// Central queue for IRQ-driven driver events.
// Flow: Interrupt -> Driver -> Event Queue -> System consumers
// =============================================================================

use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

const EVENT_QUEUE_SIZE: usize = 512;

#[derive(Clone, Copy, Debug)]
pub enum Event {
    KeyPress(char),
    MouseMove(i32, i32),
    Click,
    MouseState { x: i32, y: i32, buttons: u8 },
}

#[derive(Clone, Copy, Debug)]
pub struct TimedEvent {
    pub event: Event,
    pub irq_tick: u64,
    pub enqueue_tick: u64,
}

pub struct EventQueue {
    events: [Option<TimedEvent>; EVENT_QUEUE_SIZE],
    head: usize,
    tail: usize,
}

impl EventQueue {
    pub const fn new() -> Self {
        Self {
            events: [None; EVENT_QUEUE_SIZE],
            head: 0,
            tail: 0,
        }
    }

    #[inline]
    fn next(idx: usize) -> usize {
        (idx + 1) % EVENT_QUEUE_SIZE
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.head == self.tail
    }

    pub fn push(&mut self, event: TimedEvent) {
        self.events[self.tail] = Some(event);
        self.tail = Self::next(self.tail);
        if self.tail == self.head {
            // Drop oldest on overflow to keep IRQ path non-blocking.
            self.head = Self::next(self.head);
            DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
        PUSHED_EVENTS.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pop(&mut self) -> Option<TimedEvent> {
        if self.is_empty() {
            return None;
        }
        let ev = self.events[self.head];
        self.events[self.head] = None;
        self.head = Self::next(self.head);
        if ev.is_some() {
            POPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        }
        ev
    }
}

pub static DRIVER_EVENTS: Mutex<EventQueue> = Mutex::new(EventQueue::new());
static PUSHED_EVENTS: AtomicU64 = AtomicU64::new(0);
static POPPED_EVENTS: AtomicU64 = AtomicU64::new(0);
static DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn push_event(event: Event) {
    let tick = crate::drivers::timer::pit::get_ticks();
    let timed = TimedEvent {
        event,
        irq_tick: tick,
        enqueue_tick: tick,
    };
    if let Some(mut q) = DRIVER_EVENTS.try_lock() {
        q.push(timed);
    } else {
        // Lock contention in IRQ path: drop and continue.
        DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
}

#[inline]
pub fn pop_event() -> Option<TimedEvent> {
    let mut q = DRIVER_EVENTS.lock();
    q.pop()
}

/// Non-blocking pop — safe to call from IRQ context (uses try_lock).
#[inline]
pub fn try_pop_event() -> Option<TimedEvent> {
    let mut q = DRIVER_EVENTS.try_lock()?;
    q.pop()
}

#[derive(Clone, Copy, Debug)]
pub struct EventQueueTelemetry {
    pub pushed: u64,
    pub popped: u64,
    pub dropped: u64,
}

pub fn telemetry() -> EventQueueTelemetry {
    EventQueueTelemetry {
        pushed: PUSHED_EVENTS.load(Ordering::Relaxed),
        popped: POPPED_EVENTS.load(Ordering::Relaxed),
        dropped: DROPPED_EVENTS.load(Ordering::Relaxed),
    }
}

