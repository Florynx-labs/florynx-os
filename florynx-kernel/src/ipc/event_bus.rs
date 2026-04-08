// =============================================================================
// Florynx Kernel — IPC: Event Bus (Pub/Sub)
// =============================================================================
// System-wide async event bus. Subsystems subscribe to event types and
// receive notifications via ring buffers. Zero-copy for kernel-internal use.
// =============================================================================

use spin::Mutex;

// ---------------------------------------------------------------------------
// Event types
// ---------------------------------------------------------------------------

/// System-wide event types that subsystems can subscribe to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// Process lifecycle
    TaskSpawned,
    TaskExited,
    /// GUI events
    WindowCreated,
    WindowClosed,
    WindowFocused,
    /// Input events (from drivers → compositor)
    MouseInput,
    KeyboardInput,
    /// Filesystem
    FileOpened,
    FileClosed,
    /// IPC
    ChannelMessage,
}

/// A system event with type tag and payload.
#[derive(Debug, Clone, Copy)]
pub struct SystemEvent {
    pub event_type: EventType,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub timestamp: u64,
}

impl SystemEvent {
    pub const fn new(event_type: EventType, arg0: u64, arg1: u64, arg2: u64) -> Self {
        SystemEvent {
            event_type,
            arg0,
            arg1,
            arg2,
            timestamp: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Subscriber ring buffer
// ---------------------------------------------------------------------------

const SUBSCRIBER_RING_SIZE: usize = 64;

struct SubscriberRing {
    events: [SystemEvent; SUBSCRIBER_RING_SIZE],
    head: usize,
    tail: usize,
}

impl SubscriberRing {
    const fn new() -> Self {
        SubscriberRing {
            events: [SystemEvent::new(EventType::TaskSpawned, 0, 0, 0); SUBSCRIBER_RING_SIZE],
            head: 0,
            tail: 0,
        }
    }

    fn push(&mut self, event: SystemEvent) {
        self.events[self.tail] = event;
        self.tail = (self.tail + 1) % SUBSCRIBER_RING_SIZE;
        if self.tail == self.head {
            // Overwrite oldest
            self.head = (self.head + 1) % SUBSCRIBER_RING_SIZE;
        }
    }

    fn pop(&mut self) -> Option<SystemEvent> {
        if self.head == self.tail {
            return None;
        }
        let event = self.events[self.head];
        self.head = (self.head + 1) % SUBSCRIBER_RING_SIZE;
        Some(event)
    }

    fn is_empty(&self) -> bool {
        self.head == self.tail
    }
}

// ---------------------------------------------------------------------------
// Event Bus
// ---------------------------------------------------------------------------

const MAX_SUBSCRIPTIONS: usize = 32;

/// A subscription: which event type and which ring buffer to deliver to.
struct Subscription {
    event_type: EventType,
    /// Index into the subscriber rings array.
    ring_idx: usize,
    active: bool,
}

/// The global event bus. Supports up to MAX_SUBSCRIPTIONS active subscriptions.
pub struct EventBus {
    subscriptions: [Subscription; MAX_SUBSCRIPTIONS],
    sub_count: usize,
    rings: [SubscriberRing; MAX_SUBSCRIPTIONS],
    ring_count: usize,
}

impl EventBus {
    const fn new() -> Self {
        const EMPTY_SUB: Subscription = Subscription {
            event_type: EventType::TaskSpawned,
            ring_idx: 0,
            active: false,
        };
        const EMPTY_RING: SubscriberRing = SubscriberRing::new();

        EventBus {
            subscriptions: [EMPTY_SUB; MAX_SUBSCRIPTIONS],
            sub_count: 0,
            rings: [EMPTY_RING; MAX_SUBSCRIPTIONS],
            ring_count: 0,
        }
    }

    /// Subscribe to an event type. Returns a subscription handle (ring index)
    /// for polling events, or None if full.
    pub fn subscribe(&mut self, event_type: EventType) -> Option<usize> {
        if self.sub_count >= MAX_SUBSCRIPTIONS || self.ring_count >= MAX_SUBSCRIPTIONS {
            return None;
        }
        let ring_idx = self.ring_count;
        self.ring_count += 1;

        self.subscriptions[self.sub_count] = Subscription {
            event_type,
            ring_idx,
            active: true,
        };
        self.sub_count += 1;

        Some(ring_idx)
    }

    /// Publish an event to all subscribers of its type.
    pub fn publish(&mut self, event: SystemEvent) {
        for i in 0..self.sub_count {
            let sub = &self.subscriptions[i];
            if sub.active && sub.event_type == event.event_type {
                self.rings[sub.ring_idx].push(event);
            }
        }
    }

    /// Poll for the next event on a subscription handle.
    pub fn poll(&mut self, handle: usize) -> Option<SystemEvent> {
        if handle >= self.ring_count {
            return None;
        }
        self.rings[handle].pop()
    }

    /// Check if a subscription has pending events.
    pub fn has_pending(&self, handle: usize) -> bool {
        if handle >= self.ring_count {
            return false;
        }
        !self.rings[handle].is_empty()
    }
}

// ---------------------------------------------------------------------------
// Global instance
// ---------------------------------------------------------------------------

pub static EVENT_BUS: Mutex<EventBus> = Mutex::new(EventBus::new());

/// Publish a system event (convenience function).
pub fn publish(event: SystemEvent) {
    EVENT_BUS.lock().publish(event);
}

/// Subscribe to a system event type (convenience function).
pub fn subscribe(event_type: EventType) -> Option<usize> {
    EVENT_BUS.lock().subscribe(event_type)
}
