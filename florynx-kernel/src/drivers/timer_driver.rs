// =============================================================================
// Florynx Kernel — Timer Driver Facade
// =============================================================================

use crate::drivers::Driver;

pub struct TimerDriver;

impl TimerDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Driver for TimerDriver {
    fn init(&mut self) {
        // PIT is initialized by arch::interrupts::init().
    }

    fn handle_interrupt(&mut self) {
        crate::drivers::timer::pit::tick();
    }

    fn update(&mut self) {}
}

