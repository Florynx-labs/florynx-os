// =============================================================================
// Florynx Kernel — Keyboard Driver Facade
// =============================================================================

use crate::drivers::Driver;

pub struct KeyboardDriver;

impl KeyboardDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Driver for KeyboardDriver {
    fn init(&mut self) {
        // PS/2 keyboard is interrupt-driven and requires no explicit init here.
    }

    fn handle_interrupt(&mut self) {
        crate::drivers::input::keyboard::handle_keyboard_interrupt();
    }

    fn update(&mut self) {}
}

