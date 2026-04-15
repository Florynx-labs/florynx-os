// =============================================================================
// Florynx Kernel — GUI Dock Component
// =============================================================================
// macOS-style floating dock at the bottom of the screen.
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::theme;
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::icons::{self, Icon};
use crate::gui::animation::AnimatedScale;

const MAX_ITEMS: usize = 10;
const ICON_SIZE: usize = 48;
const ICON_GAP: usize = 16;
const HOVER_SCALE: f32 = 1.25;
const NORMAL_SCALE: f32 = 1.0;
const SCALE_SPEED: f32 = 0.18;

const MAX_ITEM_NAME: usize = 16;

#[derive(Clone, Copy)]
pub struct DockItem {
    pub color: Color,
    pub icon: &'static Icon,
    pub active: bool,
    name: [u8; MAX_ITEM_NAME],
    name_len: usize,
}

pub struct Dock {
    items: [Option<DockItem>; MAX_ITEMS],
    count: usize,
    pub hovered: Option<usize>,
    scales: [AnimatedScale; MAX_ITEMS],
}

impl Dock {
    pub const fn new() -> Self {
        Dock {
            items: [None; MAX_ITEMS],
            count: 0,
            hovered: None,
            scales: [AnimatedScale::new(NORMAL_SCALE, SCALE_SPEED); MAX_ITEMS],
        }
    }

    pub fn add(&mut self, icon: &'static Icon, color: Color) {
        self.add_named(icon, color, "");
    }

    pub fn add_named(&mut self, icon: &'static Icon, color: Color, label: &str) {
        if self.count < MAX_ITEMS {
            let mut name = [0u8; MAX_ITEM_NAME];
            let len = label.len().min(MAX_ITEM_NAME);
            name[..len].copy_from_slice(&label.as_bytes()[..len]);
            self.items[self.count] = Some(DockItem { color, icon, active: false, name, name_len: len });
            self.count += 1;
        }
    }

    fn dock_rect(&self, screen_w: usize, screen_h: usize) -> Rect {
        let t = &theme::DARK;
        let total_w = self.count * ICON_SIZE + (self.count.saturating_sub(1)) * ICON_GAP + t.padding * 2;
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

        // Glassy dock background
        // First draw a dark sheer layer, then the frosty theme layer
        renderer::draw_rounded_rect(fb, dr.x, dr.y, dr.w, dr.h, 18, Color::rgba(0, 0, 0, 80));
        renderer::draw_rounded_rect(fb, dr.x, dr.y, dr.w, dr.h, 18, t.dock_bg);
        renderer::draw_rounded_border(fb, dr.x, dr.y, dr.w, dr.h, 18, t.border);

        // Icons with scale animation
        for i in 0..self.count {
            if let Some(item) = &self.items[i] {
                let ir = self.icon_rect(i, screen_w, screen_h);
                let icon_r = 8;

                // Apply animated scale
                let scale = self.scales[i].scale.current;
                let scaled_size = (ICON_SIZE as f32 * scale) as usize;
                let offset = (scaled_size.saturating_sub(ICON_SIZE)) / 2;
                let sx = ir.x.saturating_sub(offset);
                let sy = ir.y.saturating_sub(offset);

                let color = if Some(i) == self.hovered {
                    Color::rgb(
                        item.color.r.saturating_add(30),
                        item.color.g.saturating_add(30),
                        item.color.b.saturating_add(30),
                    )
                } else {
                    item.color
                };
                renderer::draw_rounded_rect(fb, sx, sy, scaled_size, scaled_size, icon_r, color);

                // Draw icon centered in scaled rect
                let icon_x = sx + (scaled_size.saturating_sub(item.icon.width)) / 2;
                let icon_y = sy + (scaled_size.saturating_sub(item.icon.height)) / 2;
                icons::draw_icon(fb, item.icon, icon_x, icon_y, Color::WHITE);

                // Active indicator dot below icon
                if item.active {
                    let dot_x = sx + scaled_size / 2;
                    let dot_y = sy + scaled_size + 4;
                    renderer::draw_circle(fb, dot_x, dot_y, 2, t.accent);
                }
            }
        }

        // Draw tooltip above hovered icon
        if let Some(hi) = self.hovered {
            if let Some(item) = &self.items[hi] {
                if item.name_len > 0 {
                    let ir = self.icon_rect(hi, screen_w, screen_h);
                    let label = core::str::from_utf8(&item.name[..item.name_len]).unwrap_or("");
                    let text_w = label.len() * 8;
                    let pad = 8;
                    let tw = text_w + pad * 2;
                    let th = 20;
                    let tx = (ir.x + ICON_SIZE / 2).saturating_sub(tw / 2);
                    let ty = ir.y.saturating_sub(th + 6);
                    renderer::draw_rounded_rect(fb, tx, ty, tw, th, 6, t.tooltip_bg);
                    renderer::draw_rounded_border(fb, tx, ty, tw, th, 6, t.border);
                    renderer::draw_text(fb, label, tx + pad, ty + (th.saturating_sub(8)) / 2, t.tooltip_text, 1);
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> Option<usize> {
        match *event {
            Event::MouseMove { x, y } => {
                self.hovered = None;
                for i in 0..self.count {
                    if self.icon_rect(i, screen_w, screen_h).contains(x, y) {
                        self.hovered = Some(i);
                        break;
                    }
                }
                None // Don't return clicked index for hover
            }
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                // Check which icon was clicked
                for i in 0..self.count {
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
        if idx < self.count {
            if let Some(ref mut item) = self.items[idx] {
                item.active = active;
            }
        }
    }

    /// Tick dock scale animations. Returns true if any scale changed (needs redraw).
    pub fn tick_animations(&mut self) -> bool {
        // Set targets based on current hover state
        for i in 0..self.count {
            let target = if Some(i) == self.hovered { HOVER_SCALE } else { NORMAL_SCALE };
            self.scales[i].set_target(target);
        }

        // Tick all
        let mut changed = false;
        for i in 0..self.count {
            if self.scales[i].tick() {
                changed = true;
            }
        }
        changed
    }
}
