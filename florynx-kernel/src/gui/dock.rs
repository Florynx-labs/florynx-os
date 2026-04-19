// =============================================================================
// Florynx Kernel — GUI Dock Component
// =============================================================================
// macOS-style floating dock at the bottom of the screen.
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::theme;
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::dynamic_icon::DynamicIcon;
use crate::gui::animation::AnimatedScale;
use alloc::vec::Vec;

const MAX_ITEMS: usize = 10;
const ICON_SIZE: usize = 48;
const ICON_GAP: usize = 16;
const HOVER_SCALE: f32 = 1.25;
const NORMAL_SCALE: f32 = 1.0;
const SCALE_SPEED: f32 = 0.18;

const MAX_ITEM_NAME: usize = 16;

#[derive(Clone)]
pub struct DockItem {
    pub icon: DynamicIcon,
    pub active: bool,
    name: [u8; MAX_ITEM_NAME],
    name_len: usize,
}

pub struct Dock {
    items: Vec<DockItem>,
    pub hovered: Option<usize>,
    scales: [AnimatedScale; MAX_ITEMS],
}

impl Dock {
    pub fn new() -> Self {
        Dock {
            items: Vec::new(),
            hovered: None,
            scales: [AnimatedScale::new(NORMAL_SCALE, SCALE_SPEED); MAX_ITEMS],
        }
    }

    pub fn count(&self) -> usize {
        self.items.len()
    }

    pub fn add(&mut self, png_bytes: &[u8]) {
        self.add_named(png_bytes, "");
    }

    pub fn add_named(&mut self, png_bytes: &[u8], label: &str) {
        if self.items.len() < MAX_ITEMS {
            let icon = DynamicIcon::from_png_bytes(png_bytes).unwrap_or_else(|_| {
                DynamicIcon { width: 48, height: 48, rgba_data: alloc::vec![0; 48*48*4] }
            });
            let mut name = [0u8; MAX_ITEM_NAME];
            let len = label.len().min(MAX_ITEM_NAME);
            name[..len].copy_from_slice(&label.as_bytes()[..len]);
            self.items.push(DockItem { icon, active: false, name, name_len: len });
        }
    }

    fn dock_rect(&self, screen_w: usize, screen_h: usize) -> Rect {
        let t = &theme::DARK;
        let count = self.items.len();
        let total_w = count * ICON_SIZE + (count.saturating_sub(1)) * ICON_GAP + t.padding * 2;
        let dock_x = (screen_w.saturating_sub(total_w)) / 2;
        let dock_y = screen_h - t.dock_h - t.dock_margin;
        Rect::new(dock_x, dock_y, total_w, t.dock_h)
    }

    fn icon_rect(&self, idx: usize, screen_w: usize, screen_h: usize) -> Rect {
        let dr = self.dock_rect(screen_w, screen_h);
        let t = &theme::DARK;
        let ix = dr.x + t.padding + idx * (ICON_SIZE + ICON_GAP);
        let iy = dr.y + (dr.h.saturating_sub(ICON_SIZE)) / 2;
        Rect::new(ix, iy, ICON_SIZE, ICON_SIZE)
    }

    pub fn draw(&self, fb: &mut FramebufferManager, screen_w: usize, screen_h: usize) {
        let t = &theme::DARK;
        let dr = self.dock_rect(screen_w, screen_h);

        // Dock background (enhanced glassmorphism — lighter, more transparent)
        renderer::draw_rounded_rect(fb, dr.x, dr.y, dr.w, dr.h, 18,
            Color::rgba(0, 0, 0, 60));
        renderer::draw_rounded_rect(fb, dr.x, dr.y, dr.w, dr.h, 18,
            Color::rgba(25, 32, 45, 180));
        // Inner glow border for frosted glass effect
        renderer::draw_rounded_border(fb, dr.x, dr.y, dr.w, dr.h, 18,
            Color::rgba(80, 90, 110, 120));
        // Subtle inner highlight at top
        renderer::draw_hline(fb, dr.x + 18, dr.y + 1, dr.w.saturating_sub(36),
            Color::rgba(255, 255, 255, 20));

        // Icons with scale animation
        for i in 0..self.items.len() {
            let item = &self.items[i];
            let ir = self.icon_rect(i, screen_w, screen_h);

            // Apply animated scale
            let scale = self.scales[i].scale.current;
            let scaled_size = (ICON_SIZE as f32 * scale) as usize;
            let offset = (scaled_size.saturating_sub(ICON_SIZE)) / 2;
            let sx = ir.x.saturating_sub(offset);
            let sy = ir.y.saturating_sub(offset);

            // Removed solid rounded rect, letting PNG handle its own visuals/shadows
            // Draw icon centered in scaled rect
            let icon_x = sx + (scaled_size.saturating_sub(item.icon.width)) / 2;
            let icon_y = sy + (scaled_size.saturating_sub(item.icon.height)) / 2;
            
            item.icon.draw_scaled(fb, icon_x, icon_y, scale);

            // Active indicator dot below icon
            if item.active {
                let dot_x = sx + scaled_size / 2;
                let dot_y = sy + scaled_size + 4;
                renderer::draw_circle(fb, dot_x, dot_y, 2, t.accent);
            }
        }

        // Draw tooltip above hovered icon (with AA text)
        if let Some(hi) = self.hovered {
            if hi < self.items.len() {
                let item = &self.items[hi];
                if item.name_len > 0 {
                    let ir = self.icon_rect(hi, screen_w, screen_h);
                    let label = core::str::from_utf8(&item.name[..item.name_len]).unwrap_or("");
                    let text_w = renderer::measure_text_aa(label, renderer::FontSize::Normal);
                    let pad = 10;
                    let tw = text_w + pad * 2;
                    let th = 22;
                    let tx = (ir.x + ICON_SIZE / 2).saturating_sub(tw / 2);
                    let ty = ir.y.saturating_sub(th + 8);
                    renderer::draw_rounded_rect(fb, tx, ty, tw, th, 8, t.tooltip_bg);
                    renderer::draw_rounded_border(fb, tx, ty, tw, th, 8, t.border);
                    renderer::draw_text_aa(fb, label, tx + pad, ty + (th.saturating_sub(8)) / 2,
                        t.tooltip_text, renderer::FontSize::Normal);
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> Option<usize> {
        match *event {
            Event::MouseMove { x, y } => {
                self.hovered = None;
                for i in 0..self.items.len() {
                    if self.icon_rect(i, screen_w, screen_h).contains(x, y) {
                        self.hovered = Some(i);
                        break;
                    }
                }
                None // Don't return clicked index for hover
            }
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                // Check which icon was clicked
                for i in 0..self.items.len() {
                    if self.icon_rect(i, screen_w, screen_h).contains(x, y) {
                        return Some(i); // Return clicked icon index
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn set_active(&mut self, idx: usize, active: bool) {
        if idx < self.items.len() {
            self.items[idx].active = active;
        }
    }

    /// Tick dock scale animations. Returns true if any scale changed (needs redraw).
    pub fn tick_animations(&mut self) -> bool {
        // Set targets based on current hover state
        let count = self.items.len();
        for i in 0..count {
            let target = if Some(i) == self.hovered { HOVER_SCALE } else { NORMAL_SCALE };
            self.scales[i].set_target(target);
        }

        // Tick all
        let mut changed = false;
        for i in 0..count {
            if self.scales[i].tick() {
                changed = true;
            }
        }
        changed
    }
}
