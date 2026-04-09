// =============================================================================
// Florynx Kernel — Mouse Driver Facade
// =============================================================================

use crate::drivers::Driver;

pub struct MouseDriver;

impl MouseDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Driver for MouseDriver {
    fn init(&mut self) {
        let _ = crate::drivers::input::mouse::init();
    }

    fn handle_interrupt(&mut self) {
        crate::drivers::input::mouse::handle_interrupt();
    }

    fn update(&mut self) {}
}

