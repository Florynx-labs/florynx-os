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
    let caption_x = sw.saturating_sub(caption.len() * 8) / 2;
    let caption_y = (y0 + LOGO_H + 22).min(sh.saturating_sub(18));
    renderer::draw_text(fb, caption, caption_x, caption_y, Color::rgb(210, 220, 235), 1);

    fb.flush_full();
    drop(fb_guard);

    if duration_ticks == 0 {
        return;
    }
    let start = crate::time::clock::uptime_ticks();
    while crate::time::clock::uptime_ticks().saturating_sub(start) < duration_ticks {
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

