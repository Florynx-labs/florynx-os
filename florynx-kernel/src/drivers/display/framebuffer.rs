// =============================================================================
// Florynx Kernel — Framebuffer Driver
// =============================================================================
// Interfaces with video memory to perform raw pixel operations.
// Provides a safe wrapper for memory-mapped video memory.
// =============================================================================

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

/// Manages the raw framebuffer memory and provides basic drawing capabilities.
pub struct FramebufferManager {
    buffer_ptr: *mut u8,
    width: usize,
    height: usize,
    stride: usize,
    format: PixelFormat,
}

unsafe impl Send for FramebufferManager {}
unsafe impl Sync for FramebufferManager {}

impl FramebufferManager {
    /// Create a new FramebufferManager from raw parts.
    pub unsafe fn new(ptr: *mut u8, width: usize, height: usize, stride: usize, format: PixelFormat) -> Self {
        Self {
            buffer_ptr: ptr,
            width,
            height,
            stride,
            format,
        }
    }

    /// Set a pixel at the given coordinates.
    #[inline]
    pub fn set_pixel(&mut self, x: usize, y: usize, r: u8, g: u8, b: u8) {
        if x >= self.width || y >= self.height {
            return;
        }

        let pixel_offset = y * self.stride + x;
        let bytes_per_pixel = 4; // Currently only supports 32-bit
        let byte_offset = pixel_offset * bytes_per_pixel;

        unsafe {
            match self.format {
                PixelFormat::RGB => {
                    *self.buffer_ptr.add(byte_offset + 0) = r;
                    *self.buffer_ptr.add(byte_offset + 1) = g;
                    *self.buffer_ptr.add(byte_offset + 2) = b;
                }
                PixelFormat::BGR => {
                    *self.buffer_ptr.add(byte_offset + 0) = b;
                    *self.buffer_ptr.add(byte_offset + 1) = g;
                    *self.buffer_ptr.add(byte_offset + 2) = r;
                }
                PixelFormat::U8 => {
                    *self.buffer_ptr.add(byte_offset) = r;
                }
            }
        }
    }

    /// Get a pixel's RGB values at the given coordinates.
    #[inline]
    pub fn get_pixel(&self, x: usize, y: usize) -> (u8, u8, u8) {
        if x >= self.width || y >= self.height {
            return (0, 0, 0);
        }

        let pixel_offset = y * self.stride + x;
        let bytes_per_pixel = 4;
        let byte_offset = pixel_offset * bytes_per_pixel;

        unsafe {
            match self.format {
                PixelFormat::RGB => (
                    *self.buffer_ptr.add(byte_offset + 0),
                    *self.buffer_ptr.add(byte_offset + 1),
                    *self.buffer_ptr.add(byte_offset + 2),
                ),
                PixelFormat::BGR => (
                    *self.buffer_ptr.add(byte_offset + 2),
                    *self.buffer_ptr.add(byte_offset + 1),
                    *self.buffer_ptr.add(byte_offset + 0),
                ),
                PixelFormat::U8 => {
                    let v = *self.buffer_ptr.add(byte_offset);
                    (v, v, v)
                }
            }
        }
    }

    /// Clear the screen with a specific color.
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

    /// Scroll the entire framebuffer up by `pixels` rows. Clears the bottom.
    pub fn scroll_up(&mut self, pixels: usize) {
        if pixels >= self.height { self.clear(0, 0, 0); return; }
        let bpp = 4usize;
        let row_bytes = self.stride * bpp;
        let copy_h = self.height - pixels;
        unsafe {
            core::ptr::copy(
                self.buffer_ptr.add(pixels * row_bytes),
                self.buffer_ptr,
                copy_h * row_bytes,
            );
            core::ptr::write_bytes(
                self.buffer_ptr.add(copy_h * row_bytes),
                0,
                pixels * row_bytes,
            );
        }
    }
}

/// Initialize the global framebuffer.
pub unsafe fn init(ptr: *mut u8, width: usize, height: usize, stride: usize, format: PixelFormat) {
    *FRAMEBUFFER.lock() = Some(FramebufferManager::new(ptr, width, height, stride, format));
    crate::serial_println!("[gui] framebuffer initialized ({}x{} at {:?})", width, height, ptr);
}
