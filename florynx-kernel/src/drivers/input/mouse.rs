// =============================================================================
// Florynx Kernel — PS/2 Mouse Driver
// =============================================================================
// Interacts with the PS/2 controller to receive mouse motion and button events.
// Processes 3-byte packets and updates the global cursor state.
// =============================================================================

use x86_64::instructions::port::Port;
use spin::Mutex;
use lazy_static::lazy_static;

const PS2_DATA_PORT: u16 = 0x60;
const PS2_STATUS_PORT: u16 = 0x64;
const PS2_COMMAND_PORT: u16 = 0x64;

lazy_static! {
    /// Global mouse state.
    pub static ref MOUSE: Mutex<MouseState> = Mutex::new(MouseState::new());
}

pub struct MouseState {
    pub x: i32,
    pub y: i32,
    pub buttons: u8,
    cycle: u8,
    packet: [u8; 3],
}

impl MouseState {
    fn new() -> Self {
        Self {
            x: 400, // Start at center
            y: 300,
            buttons: 0,
            cycle: 0,
            packet: [0; 3],
        }
    }

    /// Process a byte from the PS/2 controller.
    pub fn process_byte(&mut self, data: u8) {
        match self.cycle {
            0 => {
                // Byte 1: Flags (must have bit 3 set)
                if data & 0x08 != 0 {
                    self.packet[0] = data;
                    self.cycle = 1;
                }
            }
            1 => {
                // Byte 2: X Delta
                self.packet[1] = data;
                self.cycle = 2;
            }
            2 => {
                // Byte 3: Y Delta
                self.packet[2] = data;
                self.cycle = 0;

                // Finished packet: Update state
                let flags = self.packet[0];
                let mut dx = self.packet[1] as i32;
                let mut dy = self.packet[2] as i32;

                // Sign extension for 9-bit deltas
                if flags & 0x10 != 0 { dx |= !0xFF; }
                if flags & 0x20 != 0 { dy |= !0xFF; }

                self.x += dx;
                self.y -= dy; // Invert Y (PS/2 is Y-up, GUI is Y-down)
                self.buttons = flags & 0x07;

                // Clamping (assumes 1024x768 for now, but we'll dynamic it in the GUI)
                if self.x < 0 { self.x = 0; }
                if self.y < 0 { self.y = 0; }
                if self.x > 1023 { self.x = 1023; }
                if self.y > 767 { self.y = 767; }

                // Trigger lightweight cursor redraw
                crate::gui::renderer::update_cursor(self.x as usize, self.y as usize);

                // Notify desktop compositor of mouse state (for event dispatch)
                crate::gui::desktop::on_mouse_update(
                    self.x as usize, self.y as usize, self.buttons
                );
            }
            _ => self.cycle = 0,
        }
    }
}

/// Initialize the PS/2 mouse. Returns false if the controller is unresponsive.
pub fn init() -> bool {
    let mut data_port = Port::new(PS2_DATA_PORT);
    let mut command_port = Port::new(PS2_COMMAND_PORT);

    unsafe {
        // Enable mouse in PS/2 controller
        if !wait_write() { crate::serial_println!("[mouse] PS/2 timeout (enable aux)"); return false; }
        command_port.write(0xA8u8); // Enable auxiliary device

        // Enable mouse interrupts
        if !wait_write() { crate::serial_println!("[mouse] PS/2 timeout (get cmd)"); return false; }
        command_port.write(0x20u8); // Get command byte
        if !wait_read() { crate::serial_println!("[mouse] PS/2 timeout (read cmd)"); return false; }
        let status: u8 = data_port.read();
        let status = status | 0x02;
        if !wait_write() { crate::serial_println!("[mouse] PS/2 timeout (set cmd)"); return false; }
        command_port.write(0x60u8); // Set command byte
        if !wait_write() { crate::serial_println!("[mouse] PS/2 timeout (write cmd)"); return false; }
        data_port.write(status);

        // Tell mouse to use default settings
        mouse_write(0xF6);
        mouse_read(); // Acknowledgement

        // Enable mouse streaming
        mouse_write(0xF4);
        mouse_read(); // Acknowledgement
    }

    crate::serial_println!("[mouse] PS/2 initialized");
    true
}

/// Maximum number of status-port polls before giving up.
const PS2_TIMEOUT: u32 = 100_000;

fn wait_write() -> bool {
    let mut status_port: Port<u8> = Port::new(PS2_STATUS_PORT);
    for _ in 0..PS2_TIMEOUT {
        unsafe {
            if status_port.read() & 0x02 == 0 {
                return true;
            }
        }
    }
    false
}

fn wait_read() -> bool {
    let mut status_port: Port<u8> = Port::new(PS2_STATUS_PORT);
    for _ in 0..PS2_TIMEOUT {
        unsafe {
            if status_port.read() & 0x01 != 0 {
                return true;
            }
        }
    }
    false
}

fn mouse_write(data: u8) {
    let mut command_port = Port::new(PS2_COMMAND_PORT);
    let mut data_port = Port::new(PS2_DATA_PORT);
    wait_write();
    unsafe {
        command_port.write(0xD4u8); // Write to mouse
        wait_write();
        data_port.write(data);
    }
}

fn mouse_read() -> u8 {
    let mut data_port = Port::new(PS2_DATA_PORT);
    wait_read();
    unsafe { data_port.read() }
}

/// IRQ 12 Handler (Mouse)
pub fn handle_interrupt() {
    let mut data_port: Port<u8> = Port::new(PS2_DATA_PORT);
    let data: u8 = unsafe { data_port.read() };
    MOUSE.lock().process_byte(data);
}
