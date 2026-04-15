// =============================================================================
// Florynx Kernel — Panic Handler
// =============================================================================
// Handles kernel panics by printing diagnostic info to serial, VGA, and framebuffer.
// =============================================================================

use core::panic::PanicInfo;
use core::fmt::Write;
use core::sync::atomic::{AtomicU64, AtomicU8, Ordering};

/// Panic policy controls terminal behavior after panic diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanicPolicy {
    Halt = 0,
    Reboot = 1,
}

static PANIC_POLICY: AtomicU8 = AtomicU8::new(PanicPolicy::Halt as u8);
static PANIC_COUNT: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy)]
pub struct PanicTelemetry {
    pub panic_count: u64,
    pub policy: PanicPolicy,
}

pub fn set_panic_policy(policy: PanicPolicy) {
    PANIC_POLICY.store(policy as u8, Ordering::Relaxed);
}

pub fn panic_telemetry() -> PanicTelemetry {
    PanicTelemetry {
        panic_count: PANIC_COUNT.load(Ordering::Relaxed),
        policy: current_policy(),
    }
}

/// The kernel panic handler — prints error info and halts.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    PANIC_COUNT.fetch_add(1, Ordering::Relaxed);

    // Disable interrupts to prevent further issues
    x86_64::instructions::interrupts::disable();

    // Print to serial (always available)
    crate::serial_println!("!!! KERNEL PANIC !!!");
    crate::serial_println!("{}", info);

    // Print to console (if active and not deadlocked)
    crate::println!("!!! KERNEL PANIC !!!");
    crate::println!("{}", info);

    // Force unlock the framebuffer so we can guarantee the red screen of death appears
    // even if the panic occurred inside GUI rendering.
    unsafe {
        crate::gui::renderer::FRAMEBUFFER.force_unlock();
    }

    // Draw panic message to framebuffer if available (visible in GUI mode)
    draw_panic_to_framebuffer(info);

    match current_policy() {
        PanicPolicy::Halt => {
            crate::serial_println!("[panic] policy=halt");
            loop {
                x86_64::instructions::hlt();
            }
        }
        PanicPolicy::Reboot => {
            crate::serial_println!("[panic] policy=reboot (kbd controller reset)");
            unsafe {
                // Standard x86 reboot trigger via keyboard controller.
                crate::arch::x86_64::asm_utils::outb(0x64, 0xFE);
            }
            // If reset does not trigger, fall back to halt.
            loop {
                x86_64::instructions::hlt();
            }
        }
    }
}

#[inline]
fn current_policy() -> PanicPolicy {
    match PANIC_POLICY.load(Ordering::Relaxed) {
        1 => PanicPolicy::Reboot,
        _ => PanicPolicy::Halt,
    }
}

/// Draw a panic message directly to the framebuffer with large red text.
fn draw_panic_to_framebuffer(info: &PanicInfo) {
    use crate::gui::renderer::{FRAMEBUFFER, Color};
    
    // Try to lock framebuffer (non-blocking to avoid deadlock in panic)
    let mut fb_guard = match FRAMEBUFFER.try_lock() {
        Some(g) => g,
        None => return, // Can't lock, skip framebuffer output
    };
    
    let fb = match fb_guard.as_mut() {
        Some(fb) => fb,
        None => return, // No framebuffer initialized
    };

    let (width, height) = fb.dimensions();
    
    // Fill screen with dark red background
    let bg = Color::rgb(40, 0, 0);
    for y in 0..height {
        for x in 0..width {
            fb.set_pixel(x, y, bg.r, bg.g, bg.b);
        }
    }

    // Draw panic header
    let header_color = Color::rgb(255, 80, 80);
    let x_start = 50;
    let mut y = 50;
    
    crate::gui::renderer::draw_text(fb, "!!! KERNEL PANIC !!!", x_start, y, header_color, 1);
    y += 20;
    
    crate::gui::renderer::draw_text(fb, "The kernel encountered a fatal error and must halt.", x_start, y, Color::rgb(200, 200, 200), 1);
    y += 30;

    // Extract panic message
    let text_color = Color::rgb(255, 255, 255);
    
    // Format the panic info into a string buffer
    let mut buffer = PanicBuffer::new();
    let _ = write!(&mut buffer, "{}", info);
    
    // Draw each line of the panic message
    for line in buffer.as_str().lines() {
        if y + 10 > height { break; } // Stop if we run out of screen space
        crate::gui::renderer::draw_text(fb, line, x_start, y, text_color, 1);
        y += 12;
    }
    
    // Draw footer
    y = height.saturating_sub(40);
    crate::gui::renderer::draw_text(fb, "System halted. Please reboot.", x_start, y, Color::rgb(150, 150, 150), 1);
}

/// Simple fixed-size buffer for formatting panic messages.
struct PanicBuffer {
    buffer: [u8; 512],
    len: usize,
}

impl PanicBuffer {
    fn new() -> Self {
        PanicBuffer {
            buffer: [0; 512],
            len: 0,
        }
    }
    
    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buffer[..self.len]).unwrap_or("<invalid utf8>")
    }
}

impl Write for PanicBuffer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let bytes = s.as_bytes();
        let remaining = self.buffer.len() - self.len;
        let to_write = bytes.len().min(remaining);
        
        self.buffer[self.len..self.len + to_write].copy_from_slice(&bytes[..to_write]);
        self.len += to_write;
        
        Ok(())
    }
}
