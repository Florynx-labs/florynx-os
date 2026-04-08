// =============================================================================
// Florynx Kernel — Framebuffer Driver (Double-Buffered)
// =============================================================================
// All drawing goes to a RAM back buffer. Dirty regions are flushed to the
// hardware VRAM front buffer via fast memcpy, eliminating per-pixel MMIO
// writes and screen tearing.
// =============================================================================

use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

lazy_static! {
    /// The global framebuffer instance, protected by a mutex.
    pub static ref FRAMEBUFFER: Mutex<Option<FramebufferManager>> = Mutex::new(None);
}

/// Pixel formats supported by the renderer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    RGB,
    BGR,
    U8,
}

/// Manages a double-buffered framebuffer:
/// - `back`: heap-allocated RAM buffer (fast writes)
/// - `front_ptr`: memory-mapped VRAM (slow MMIO writes)
/// All `set_pixel` calls write to `back`. Call `flush_rect` to copy
/// a dirty region to VRAM, or `flush_full` to copy everything.
pub struct FramebufferManager {
    front_ptr: *mut u8,
    back: Vec<u8>,
    width: usize,
    height: usize,
    stride: usize,
    format: PixelFormat,
}

unsafe impl Send for FramebufferManager {}
unsafe impl Sync for FramebufferManager {}

impl FramebufferManager {
    /// Create a new double-buffered FramebufferManager.
    pub unsafe fn new(ptr: *mut u8, width: usize, height: usize, stride: usize, format: PixelFormat) -> Self {
        let total_bytes = height * stride * 4;
        let back = alloc::vec![0u8; total_bytes];
        Self {
            front_ptr: ptr,
            back,
            width,
            height,
            stride,
            format,
        }
    }

    /// Set a pixel in the back buffer (fast — RAM only).
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        let byte_offset = (y * self.stride + x) * 4;
        match self.format {
            PixelFormat::RGB => {
                self.back[byte_offset]     = r;
                self.back[byte_offset + 1] = g;
                self.back[byte_offset + 2] = b;
            }
            PixelFormat::BGR => {
                self.back[byte_offset]     = b;
                self.back[byte_offset + 1] = g;
                self.back[byte_offset + 2] = r;
            }
            PixelFormat::U8 => {
                self.back[byte_offset] = r;
            }
        }
    }

    /// Read a pixel from the back buffer.
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0);
        }
        let byte_offset = (y * self.stride + x) * 4;
        match self.format {
            PixelFormat::RGB => (
                self.back[byte_offset],
                self.back[byte_offset + 1],
                self.back[byte_offset + 2],
            ),
            PixelFormat::BGR => (
                self.back[byte_offset + 2],
                self.back[byte_offset + 1],
                self.back[byte_offset],
            ),
            PixelFormat::U8 => {
                let v = self.back[byte_offset];
                (v, v, v)
            }
        }
    }

    /// Flush a rectangular region from back buffer to VRAM.
    /// This is the core of damage-based rendering — only dirty pixels
    /// are copied over the slow MMIO bus.
    pub fn flush_rect(&mut self, x: usize, y: usize, w: usize, h: usize) {
        let x = x.min(self.width);
        let y = y.min(self.height);
        let x2 = (x + w).min(self.width);
        let y2 = (y + h).min(self.height);
        if x2 <= x || y2 <= y { return; }

        let row_bytes = (x2 - x) * 4;
        for row in y..y2 {
            let offset = (row * self.stride + x) * 4;
            unsafe {
                core::ptr::copy_nonoverlapping(
                    self.back.as_ptr().add(offset),
                    self.front_ptr.add(offset),
                    row_bytes,
                );
            }
        }
    }

    /// Flush the entire back buffer to VRAM.
    pub fn flush_full(&mut self) {
        let total = self.height * self.stride * 4;
        unsafe {
            core::ptr::copy_nonoverlapping(
                self.back.as_ptr(),
                self.front_ptr,
                total,
            );
        }
    }

    /// Clear the back buffer with a specific color.
    pub fn clear(&mut self, r: u8, g: u8, b: u8) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.set_pixel(x, y, r, g, b);
            }
        }
    }

    /// Returns the screen dimensions.
    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    /// Screen width in pixels.
    pub fn width(&self) -> usize { self.width }

    /// Screen height in pixels.
    pub fn height(&self) -> usize { self.height }

    /// Scroll the back buffer up by `pixels` rows. Clears the bottom.
    pub fn scroll_up(&mut self, pixels: usize) {
        if pixels >= self.height { self.clear(0, 0, 0); return; }
        let row_bytes = self.stride * 4;
        let copy_h = self.height - pixels;
        let src_offset = pixels * row_bytes;
        self.back.copy_within(src_offset..src_offset + copy_h * row_bytes, 0);
        let clear_start = copy_h * row_bytes;
        for b in &mut self.back[clear_start..clear_start + pixels * row_bytes] {
            *b = 0;
        }
    }
}

/// Initialize the global framebuffer with double buffering.
pub unsafe fn init(ptr: *mut u8, width: usize, height: usize, stride: usize, format: PixelFormat) {
    *FRAMEBUFFER.lock() = Some(FramebufferManager::new(ptr, width, height, stride, format));
    crate::serial_println!("[gui] framebuffer initialized ({}x{} at {:?})", width, height, ptr);
}
