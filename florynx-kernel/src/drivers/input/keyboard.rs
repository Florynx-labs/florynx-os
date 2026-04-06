// =============================================================================
// Florynx Kernel — PS/2 Keyboard Driver
// =============================================================================
// Handles keyboard input via IRQ1 and the PS/2 controller at port 0x60.
// Decodes scancodes into ASCII characters using the pc-keyboard crate.
// =============================================================================

use lazy_static::lazy_static;
use pc_keyboard::{layouts, DecodedKey, HandleControl, Keyboard, ScancodeSet1};
use spin::Mutex;
use x86_64::instructions::port::Port;

lazy_static! {
    /// Global keyboard decoder state.
    static ref KEYBOARD: Mutex<Keyboard<layouts::Us104Key, ScancodeSet1>> = Mutex::new(
        Keyboard::new(
            ScancodeSet1::new(),
            layouts::Us104Key,
            HandleControl::Ignore,
        )
    );
}

/// Called by the keyboard interrupt handler (IRQ1).
/// Reads the scancode from port 0x60 and decodes it.
pub fn handle_keyboard_interrupt() {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            match key {
                DecodedKey::Unicode(character) => {
                    crate::print!("{}", character);
                }
                DecodedKey::RawKey(key) => {
                    crate::serial_println!("[keyboard] raw key: {:?}", key);
                }
            }
        }
    }
}
