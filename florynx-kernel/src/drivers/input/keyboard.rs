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
            HandleControl::MapLettersToUnicode,
        )
    );
}

/// Initialize PS/2 keyboard: set fastest typematic rate (250 ms delay, 30 cps).
pub fn init_typematic() {
    let mut status_port: Port<u8> = Port::new(0x64);
    let mut data_port:   Port<u8> = Port::new(0x60);
    unsafe {
        // Wait for input buffer empty, then send 0xF3 (Set Typematic Rate/Delay)
        for _ in 0..100_000u32 { if status_port.read() & 0x02 == 0 { break; } }
        data_port.write(0xF3u8);
        // Wait for ACK
        for _ in 0..100_000u32 { if status_port.read() & 0x01 != 0 { break; } }
        let _ = data_port.read();
        // Wait for input buffer empty, then send rate byte 0x00 = 250ms / 30 cps
        for _ in 0..100_000u32 { if status_port.read() & 0x02 == 0 { break; } }
        data_port.write(0x00u8);
        // Wait for ACK
        for _ in 0..100_000u32 { if status_port.read() & 0x01 != 0 { break; } }
        let _ = data_port.read();
    }
    crate::serial_println!("[keyboard] typematic rate set: 250ms delay, 30 cps");
}

/// Called by the keyboard interrupt handler (IRQ1).
/// Reads the scancode from port 0x60 and decodes it.
pub fn handle_keyboard_interrupt() {
    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    let mut keyboard = KEYBOARD.lock();
    if let Ok(Some(key_event)) = keyboard.add_byte(scancode) {
        if let Some(key) = keyboard.process_keyevent(key_event) {
            // Convert pc-keyboard key to our GUI Key enum
            let gui_key = match key {
                DecodedKey::Unicode(character) => {
                    match character {
                        '\x08' => crate::gui::event::Key::Backspace,
                        '\n' | '\r' => crate::gui::event::Key::Enter,
                        '\t' => crate::gui::event::Key::Tab,
                        '\x1b' => crate::gui::event::Key::Escape,
                        c => crate::gui::event::Key::Char(c),
                    }
                }
                DecodedKey::RawKey(raw) => {
                    use pc_keyboard::KeyCode;
                    match raw {
                        KeyCode::ArrowUp => crate::gui::event::Key::ArrowUp,
                        KeyCode::ArrowDown => crate::gui::event::Key::ArrowDown,
                        KeyCode::ArrowLeft => crate::gui::event::Key::ArrowLeft,
                        KeyCode::ArrowRight => crate::gui::event::Key::ArrowRight,
                        KeyCode::Delete => crate::gui::event::Key::Delete,
                        KeyCode::Home => crate::gui::event::Key::Home,
                        KeyCode::End => crate::gui::event::Key::End,
                        KeyCode::PageUp => crate::gui::event::Key::PageUp,
                        KeyCode::PageDown => crate::gui::event::Key::PageDown,
                        // Silently ignore modifier keys and other non-character keys
                        KeyCode::LShift | KeyCode::RShift |
                        KeyCode::LControl | KeyCode::RControl |
                        KeyCode::LAlt | KeyCode::RAltGr |
                        KeyCode::LWin | KeyCode::RWin |
                        KeyCode::CapsLock | KeyCode::NumpadLock | KeyCode::ScrollLock |
                        KeyCode::F1 | KeyCode::F2 | KeyCode::F3 | KeyCode::F4 |
                        KeyCode::F5 | KeyCode::F6 | KeyCode::F7 | KeyCode::F8 |
                        KeyCode::F9 | KeyCode::F10 | KeyCode::F11 | KeyCode::F12 |
                        KeyCode::Insert | KeyCode::PauseBreak | KeyCode::PrintScreen => {
                            return; // Silently ignore
                        }
                        _ => {
                            // Other unhandled keys are ignored to keep IRQ path fast.
                            return;
                        }
                    }
                }
            };

            // Route keys: char-type keys through the driver event queue (for
            // latency telemetry); special keys go directly to the GUI event bus
            // because the driver event queue only carries char-encoded events.
            match gui_key {
                crate::gui::event::Key::Char(c) => {
                    crate::drivers::event::push_event(crate::drivers::event::Event::KeyPress(c));
                }
                crate::gui::event::Key::Backspace => {
                    crate::drivers::event::push_event(crate::drivers::event::Event::KeyPress('\x08'));
                }
                crate::gui::event::Key::Enter => {
                    crate::drivers::event::push_event(crate::drivers::event::Event::KeyPress('\n'));
                }
                crate::gui::event::Key::Tab => {
                    crate::drivers::event::push_event(crate::drivers::event::Event::KeyPress('\t'));
                }
                crate::gui::event::Key::Escape => {
                    crate::drivers::event::push_event(crate::drivers::event::Event::KeyPress('\x1b'));
                }
                special => {
                    // Arrow keys, Delete, Home, End, PageUp, PageDown bypass the
                    // driver event queue and go directly to the GUI input bus.
                    crate::gui::event_bus::push_key_press(special);
                }
            }
        }
    }
}
