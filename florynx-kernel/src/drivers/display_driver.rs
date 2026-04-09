// =============================================================================
// Florynx Kernel — Display Driver Facade
// =============================================================================

use crate::drivers::Driver;

pub struct DisplayDriver;

impl DisplayDriver {
    pub const fn new() -> Self {
        Self
    }
}

impl Driver for DisplayDriver {
    fn init(&mut self) {
        // Display is initialized in kernel init sequence via BGA setup.
    }

    fn handle_interrupt(&mut self) {
        // No display IRQ path in current bring-up.
    }

    fn update(&mut self) {
        // Future: deferred display work / mode setting updates.
    }
}

