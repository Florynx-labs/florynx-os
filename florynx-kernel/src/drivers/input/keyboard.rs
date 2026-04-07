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
                            // Other unhandled keys - log for debugging
                            crate::serial_println!("[keyboard] unhandled raw key: {:?}", raw);
                            return;
                        }
                    }
                }
            };

            // Dispatch key event to desktop
            crate::gui::desktop::on_key_press(gui_key);
        }
    }
}
