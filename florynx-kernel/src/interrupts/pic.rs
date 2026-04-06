// =============================================================================
// Florynx Kernel — PIC (8259) Interrupt Controller
// =============================================================================
// Manages the chained 8259 PIC for hardware IRQ routing.
// Maps IRQs 0-15 to interrupt vectors 32-47.
// =============================================================================

use lazy_static::lazy_static;
use pic8259::ChainedPics;
use spin::Mutex;

/// Offset for the primary PIC (IRQ 0-7 → vectors 32-39).
pub const PIC_1_OFFSET: u8 = 32;
/// Offset for the secondary PIC (IRQ 8-15 → vectors 40-47).
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

lazy_static! {
    /// Global chained PIC instance.
    pub static ref PICS: Mutex<ChainedPics> =
        Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });
}

/// Hardware interrupt index mapping.
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard = PIC_1_OFFSET + 1,
    Mouse = PIC_2_OFFSET + 4, // IRQ 12
}

impl InterruptIndex {
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    pub fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

/// Initialize the chained PICs.
pub fn init() {
    unsafe {
        PICS.lock().initialize();
    }
    crate::serial_println!("[pic] chained PICs initialized (offsets {}, {})", PIC_1_OFFSET, PIC_2_OFFSET);
}
