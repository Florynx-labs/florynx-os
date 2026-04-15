// =============================================================================
// Florynx Kernel — Drivers Module
// =============================================================================
// Top-level module for all hardware drivers.
// =============================================================================

pub mod display;
pub mod serial;
pub mod input;
pub mod timer;
pub mod timer_driver;
pub mod display_driver;
pub mod event;
pub mod keyboard;
pub mod mouse;
pub mod disk;
pub mod block;
pub mod pci;

use lazy_static::lazy_static;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

/// Lightweight driver interface used by IRQ-facing subsystems.
pub trait Driver {
    fn init(&mut self);
    fn handle_interrupt(&mut self);
    fn update(&mut self);
}

lazy_static! {
    // Split lock domains for low-latency IRQ handling.
    static ref KEYBOARD_DRIVER: Mutex<keyboard::KeyboardDriver> = Mutex::new(keyboard::KeyboardDriver::new());
    static ref MOUSE_DRIVER: Mutex<mouse::MouseDriver> = Mutex::new(mouse::MouseDriver::new());
    static ref TIMER_DRIVER: Mutex<timer_driver::TimerDriver> = Mutex::new(timer_driver::TimerDriver::new());
    static ref DISPLAY_DRIVER: Mutex<display_driver::DisplayDriver> = Mutex::new(display_driver::DisplayDriver::new());
    static ref DISK_DRIVER: Mutex<disk::DiskDriver> = Mutex::new(disk::DiskDriver::new());
}

static MAX_IRQ_TO_CONSUME_TICKS: AtomicU64 = AtomicU64::new(0);
static MAX_ENQUEUE_TO_CONSUME_TICKS: AtomicU64 = AtomicU64::new(0);

/// Initialize core drivers in deterministic order.
pub fn init_registry() {
    TIMER_DRIVER.lock().init();
    KEYBOARD_DRIVER.lock().init();
    // Mouse init stays in dedicated boot path to preserve established ordering.
    DISPLAY_DRIVER.lock().init();
    DISK_DRIVER.lock().init();
}

#[inline]
pub fn handle_keyboard_irq() {
    if let Some(mut d) = KEYBOARD_DRIVER.try_lock() {
        d.handle_interrupt();
    } else {
        // Fallback keeps input alive under short lock contention.
        crate::drivers::input::keyboard::handle_keyboard_interrupt();
    }
}

#[inline]
pub fn handle_mouse_irq() {
    if let Some(mut d) = MOUSE_DRIVER.try_lock() {
        d.handle_interrupt();
    } else {
        crate::drivers::input::mouse::handle_interrupt();
    }
}

#[inline]
pub fn handle_timer_irq() {
    if let Some(mut d) = TIMER_DRIVER.try_lock() {
        d.handle_interrupt();
    } else {
        crate::drivers::timer::pit::tick();
    }
}

/// Drain driver events and forward them to system consumers.
pub fn process_events() {
    use crate::drivers::event::{pop_event, Event};
    use crate::gui::event::Key;

    while let Some(ev) = pop_event() {
        let now = crate::drivers::timer::pit::get_ticks();
        update_max(&MAX_IRQ_TO_CONSUME_TICKS, now.saturating_sub(ev.irq_tick));
        update_max(&MAX_ENQUEUE_TO_CONSUME_TICKS, now.saturating_sub(ev.enqueue_tick));

        match ev.event {
            Event::KeyPress(c) => {
                let key = match c {
                    '\x08' => Key::Backspace,
                    '\n' | '\r' => Key::Enter,
                    '\t' => Key::Tab,
                    '\x1b' => Key::Escape,
                    ch => Key::Char(ch),
                };
                crate::gui::event_bus::push_key_press(key);
            }
            Event::MouseState { x, y, buttons } => {
                if x >= 0 && y >= 0 {
                    crate::gui::event_bus::push_mouse_state(x as usize, y as usize, buttons);
                }
            }
            Event::MouseMove(x, y) => {
                if x >= 0 && y >= 0 {
                    crate::gui::event_bus::push_mouse_state(x as usize, y as usize, 0);
                }
            }
            Event::Click => {
                // Click is represented in MouseState with buttons in current pipeline.
            }
        }
    }
}

/// Non-blocking variant of `process_events` — safe to call from timer ISR.
/// Uses try_lock to avoid deadlock if the main loop holds the event queue lock.
pub fn try_process_events() {
    use crate::drivers::event::{try_pop_event, Event};
    use crate::gui::event::Key;

    // Drain up to 32 events per call to keep ISR time bounded.
    for _ in 0..32 {
        let ev = match try_pop_event() {
            Some(e) => e,
            None => break,
        };

        match ev.event {
            Event::KeyPress(c) => {
                let key = match c {
                    '\x08' => Key::Backspace,
                    '\n' | '\r' => Key::Enter,
                    '\t' => Key::Tab,
                    '\x1b' => Key::Escape,
                    ch => Key::Char(ch),
                };
                crate::gui::event_bus::push_key_press(key);
            }
            Event::MouseState { x, y, buttons } => {
                if x >= 0 && y >= 0 {
                    crate::gui::event_bus::push_mouse_state(x as usize, y as usize, buttons);
                }
            }
            Event::MouseMove(x, y) => {
                if x >= 0 && y >= 0 {
                    crate::gui::event_bus::push_mouse_state(x as usize, y as usize, 0);
                }
            }
            Event::Click => {}
        }
    }
}

/// Run deferred non-IRQ driver work.
pub fn update_deferred() {
    if let Some(mut d) = DISPLAY_DRIVER.try_lock() {
        d.update();
    }
    if let Some(mut d) = DISK_DRIVER.try_lock() {
        d.update();
    }
}

#[inline]
fn update_max(target: &AtomicU64, value: u64) {
    let mut cur = target.load(Ordering::Relaxed);
    while value > cur {
        match target.compare_exchange_weak(cur, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(next) => cur = next,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DriverTelemetry {
    pub max_irq_to_consume_ticks: u64,
    pub max_enqueue_to_consume_ticks: u64,
}

pub fn telemetry() -> DriverTelemetry {
    DriverTelemetry {
        max_irq_to_consume_ticks: MAX_IRQ_TO_CONSUME_TICKS.load(Ordering::Relaxed),
        max_enqueue_to_consume_ticks: MAX_ENQUEUE_TO_CONSUME_TICKS.load(Ordering::Relaxed),
    }
}
