// =============================================================================
// Florynx Kernel — Dynamic Icon Engine
// =============================================================================
// Supports parsing and drawing PNG images with alpha channel in Ring 0.
// =============================================================================

use crate::gui::renderer::{Color, FramebufferManager};
use crate::gui::font::alpha_blend;
extern crate alloc;
use alloc::vec::Vec;
use png_decoder::decode;

#[derive(Clone)]
pub struct DynamicIcon {
    pub width: usize,
    pub height: usize,
    pub rgba_data: Vec<u8>,
}

impl DynamicIcon {
    /// Loads a PNG from raw bytes without requiring std::io.
    pub fn from_png_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let (header, decoded_pixels) = decode(bytes).map_err(|_| "Failed to decode PNG")?;
        
        let width = header.width as usize;
        let height = header.height as usize;
        
        // Convert Vec<[u8; 4]> to Vec<u8> flattened
        let mut rgba_data = Vec::with_capacity(width * height * 4);
        for pixel in decoded_pixels {
            rgba_data.extend_from_slice(&pixel);
        }
        
        Ok(Self {
            width,
            height,
            rgba_data,
        })
    }

    /// Blit the loaded PNG to the framebuffer with alpha blending support.
    pub fn draw(&self, fb: &mut FramebufferManager, x: usize, y: usize) {
        let (sw, sh) = fb.dimensions();
        let bytes_per_pixel = 4;
        
        for py in 0..self.height {
            let ry = y + py;
            if ry >= sh { continue; }
            
            for px in 0..self.width {
                let rx = x + px;
                if rx >= sw { continue; }
                
                let idx = (py * self.width + px) * bytes_per_pixel;
                if idx + 3 < self.rgba_data.len() {
                    let r = self.rgba_data[idx];
                    let g = self.rgba_data[idx + 1];
                    let b = self.rgba_data[idx + 2];
                    let a = self.rgba_data[idx + 3];
                    
                    if a == 255 {
                        fb.set_pixel(rx, ry, r, g, b);
                    } else if a > 0 {
                        let (bg_r, bg_g, bg_b) = fb.get_pixel(rx, ry);
                        let (nr, ng, nb) = alpha_blend(Color::rgba(r, g, b, 255), bg_r, bg_g, bg_b, a);
                        fb.set_pixel(rx, ry, nr, ng, nb);
                    }
                }
            }
        }
    }

    /// Blit the loaded PNG to the framebuffer scaled to fit target dimensions.
    pub fn draw_scaled(&self, fb: &mut FramebufferManager, x: usize, y: usize, target_w: usize, target_h: usize) {
        if target_w == self.width && target_h == self.height {
            self.draw(fb, x, y);
            return;
        }
        
        let (sw, sh) = fb.dimensions();
        let bytes_per_pixel = 4;
        
        let scale_x = target_w as f32 / self.width as f32;
        let scale_y = target_h as f32 / self.height as f32;
        
        for py in 0..target_h {
            let ry = y as i32 + py as i32;
            if ry < 0 || ry >= sh as i32 { continue; }
            let fry = ry as usize;
            
            let orig_y = ((py as f32) / scale_y) as usize;
            if orig_y >= self.height { continue; }
            
            for px in 0..target_w {
                let rx = x as i32 + px as i32;
                if rx < 0 || rx >= sw as i32 { continue; }
                let frx = rx as usize;
                
                let orig_x = ((px as f32) / scale_x) as usize;
                if orig_x >= self.width { continue; }
                
                let idx = (orig_y * self.width + orig_x) * bytes_per_pixel;
                if idx + 3 < self.rgba_data.len() {
                    let r = self.rgba_data[idx];
                    let g = self.rgba_data[idx + 1];
                    let b = self.rgba_data[idx + 2];
                    let a = self.rgba_data[idx + 3];
                    
                    if a == 255 {
                        fb.set_pixel(frx, fry, r, g, b);
                    } else if a > 0 {
                        let (bg_r, bg_g, bg_b) = fb.get_pixel(frx, fry);
                        let (nr, ng, nb) = alpha_blend(Color::rgba(r, g, b, 255), bg_r, bg_g, bg_b, a);
                        fb.set_pixel(frx, fry, nr, ng, nb);
                    }
                }
            }
        }
    }
}
