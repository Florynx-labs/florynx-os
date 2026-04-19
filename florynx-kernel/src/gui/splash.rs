// =============================================================================
// Florynx Kernel — Startup Splash
// =============================================================================
// Shows the Florynx logo centered at startup before HGUI shell surfaces appear.
// =============================================================================

use crate::gui::renderer::{self, Color, FRAMEBUFFER};

const LOGO_W: usize = 640;
const LOGO_H: usize = 425;
const LOGO_RGBA: &[u8] = include_bytes!("assets/logo_640x425.rgba");

pub fn show_startup_logo(duration_ticks: u64) {
    let mut fb_guard = FRAMEBUFFER.lock();
    let fb = match fb_guard.as_mut() {
        Some(fb) => fb,
        None => return,
    };

    let (sw, sh) = fb.dimensions();
    renderer::draw_gradient_with_noise(
        fb,
        0,
        0,
        sw,
        sh,
        Color::rgb(10, 16, 30),
        Color::rgb(7, 11, 24),
        4,
    );
    renderer::draw_vignette(fb, 24);

    let x0 = sw.saturating_sub(LOGO_W) / 2;
    let y0 = sh.saturating_sub(LOGO_H) / 2;
    blit_rgba(fb, x0, y0, LOGO_W, LOGO_H, LOGO_RGBA);

    let caption = "Starting Florynx HGUI...";
    let text_w = renderer::measure_text_aa(caption, renderer::FontSize::Normal);
    let caption_x = sw.saturating_sub(text_w) / 2;
    let caption_y = (y0 + LOGO_H + 22).min(sh.saturating_sub(18));
    
    // Animation loop
    let duration_ticks = duration_ticks.max(1); // Force at least one frame
    let start = crate::time::clock::uptime_ticks();
    
    loop {
        let now = crate::time::clock::uptime_ticks();
        let elapsed = now.saturating_sub(start);
        if elapsed >= duration_ticks { break; }
        
        // Re-draw background area around animation if needed, 
        // but since we don't have partial redraw here, we'll redraw the whole text/spinner area.
        // Actually, for a splash screen, we can just redraw everything if it's not too slow.
        // Or just the caption area.
        
        // Draw caption with Roboto
        renderer::draw_text_aa(fb, caption, caption_x, caption_y, Color::rgb(210, 220, 235), renderer::FontSize::Normal);
        
        // Draw Spinner (8 dots in a circle)
        let spinner_cx = sw / 2;
        let spinner_cy = caption_y + 30;
        let radius = 12.0;
        let dot_count = 8;
        
        for i in 0..dot_count {
            let angle = (i as f32) * (core::f32::consts::PI * 2.0) / (dot_count as f32);
            let dx = (libm::cosf(angle) * radius) as i32;
            let dy = (libm::sinf(angle) * radius) as i32;
            
            // Pulsating brightness based on time
            let phase = (elapsed as f32 * 0.1) - (i as f32 * 0.8);
            let brightness = ((libm::sinf(phase) + 1.0) * 0.5 * 255.0) as u8;
            let dot_color = Color::rgb(brightness.max(40), brightness.max(60), brightness.max(100));
            
            renderer::draw_circle(fb, (spinner_cx as i32 + dx) as usize, (spinner_cy as i32 + dy) as usize, 2, dot_color);
        }
        
        fb.flush_full();
        x86_64::instructions::hlt();
    }
}

fn blit_rgba(
    fb: &mut crate::drivers::display::framebuffer::FramebufferManager,
    x0: usize,
    y0: usize,
    w: usize,
    h: usize,
    rgba: &[u8],
) {
    let expected = w * h * 4;
    if rgba.len() < expected {
        return;
    }
    let (sw, sh) = fb.dimensions();
    for y in 0..h {
        let dy = y0 + y;
        if dy >= sh {
            break;
        }
        for x in 0..w {
            let dx = x0 + x;
            if dx >= sw {
                break;
            }
            let i = (y * w + x) * 4;
            let a = rgba[i + 3] as u16;
            if a == 0 {
                continue;
            }
            if a == 255 {
                fb.set_pixel(dx, dy, rgba[i], rgba[i + 1], rgba[i + 2]);
            } else {
                let (bg_r, bg_g, bg_b) = fb.get_pixel(dx, dy);
                let inv = 255 - a;
                let r = ((rgba[i] as u16 * a + bg_r as u16 * inv) / 255) as u8;
                let g = ((rgba[i + 1] as u16 * a + bg_g as u16 * inv) / 255) as u8;
                let b = ((rgba[i + 2] as u16 * a + bg_b as u16 * inv) / 255) as u8;
                fb.set_pixel(dx, dy, r, g, b);
            }
        }
    }
}

