// =============================================================================
// Florynx Kernel — GUI Dock Component
// =============================================================================
// macOS-style floating dock at the bottom of the screen.
// =============================================================================

use crate::gui::renderer::{self, Color, FramebufferManager};
use crate::gui::theme;
use crate::gui::event::{Event, MouseButton, Rect};
use crate::gui::icons::{self, Icon};

const MAX_ITEMS: usize = 10;
const ICON_SIZE: usize = 36;
const ICON_GAP: usize = 10;

#[derive(Clone, Copy)]
pub struct DockItem {
    pub color: Color,
    pub icon: &'static Icon,
    pub active: bool,
}

pub struct Dock {
    items: [Option<DockItem>; MAX_ITEMS],
    count: usize,
    pub hovered: Option<usize>,
}

impl Dock {
    pub const fn new() -> Self {
        Dock {
            items: [None; MAX_ITEMS],
            count: 0,
            hovered: None,
        }
    }

    pub fn add(&mut self, icon: &'static Icon, color: Color) {
        if self.count < MAX_ITEMS {
            self.items[self.count] = Some(DockItem { color, icon, active: false });
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

        // Dock background (rounded, semi-transparent dark)
        renderer::draw_rounded_rect(fb, dr.x, dr.y, dr.w, dr.h, 14, t.dock_bg);
        renderer::draw_rounded_border(fb, dr.x, dr.y, dr.w, dr.h, 14, t.border);

        // Icons
        for i in 0..self.count {
            if let Some(item) = &self.items[i] {
                let ir = self.icon_rect(i, screen_w, screen_h);
                let icon_r = 8;
                let color = if Some(i) == self.hovered {
                    // Brighten on hover
                    Color::rgb(
                        item.color.r.saturating_add(30),
                        item.color.g.saturating_add(30),
                        item.color.b.saturating_add(30),
                    )
                } else {
                    item.color
                };
                renderer::draw_rounded_rect(fb, ir.x, ir.y, ir.w, ir.h, icon_r, color);

                // Draw icon centered
                let icon_x = ir.x + (ICON_SIZE.saturating_sub(item.icon.width)) / 2;
                let icon_y = ir.y + (ICON_SIZE.saturating_sub(item.icon.height)) / 2;
                icons::draw_icon(fb, item.icon, icon_x, icon_y, Color::WHITE);

                // Active indicator dot below icon
                if item.active {
                    let dot_x = ir.x + ICON_SIZE / 2;
                    let dot_y = ir.y + ICON_SIZE + 4;
                    renderer::draw_circle(fb, dot_x, dot_y, 2, t.accent);
                }
            }
        }
    }

    pub fn handle_event(&mut self, event: &Event, screen_w: usize, screen_h: usize) -> bool {
        match *event {
            Event::MouseMove { x, y } => {
                let old = self.hovered;
                self.hovered = None;
                for i in 0..self.count {
                    if self.icon_rect(i, screen_w, screen_h).contains(x, y) {
                        self.hovered = Some(i);
                        break;
                    }
                }
                self.hovered != old // consumed if changed
            }
            Event::MouseDown { x, y, button: MouseButton::Left } => {
                let dr = self.dock_rect(screen_w, screen_h);
                dr.contains(x, y) // consume clicks on dock
            }
            _ => false,
        }
    }

    pub fn set_active(&mut self, idx: usize, active: bool) {
        if idx < self.count {
            if let Some(ref mut item) = self.items[idx] {
                item.active = active;
            }
        }
    }
}
