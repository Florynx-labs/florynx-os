// =============================================================================
// Florynx Userland — Desktop Shell (KDE Plasma-Inspired)
// =============================================================================
// Main compositor shell. Manages:
//   - Wallpaper layer (bottom)
//   - Window layer (middle, z-ordered)
//   - Panel layer (top)
//   - App menu overlay
//   - Cursor (topmost)
//
// Architecture matches KDE Plasma's layered approach:
//   Desktop → Containments → Panels → Widgets
// =============================================================================

use super::panel::Panel;
use super::app_menu::AppMenu;
use super::taskbar::Taskbar;
use super::systray::SystemTray;
use super::api::{self, GuiEventV1};
use super::wallpaper::WallpaperManager;
use super::ui;
use super::ui::widget::Widget;
use super::ui::widgets::{Button, Container, ContainerKind, Input, Text};
use florynx_shared::types::{HGUI_PANEL_TITLE, HGUI_SCREEN_W};
use alloc::boxed::Box;

/// Desktop shell state — the top-level userland GUI manager.
pub struct DesktopShell {
    pub screen_w: usize,
    pub screen_h: usize,

    // Layers (bottom to top)
    pub wallpaper: WallpaperManager,
    pub panel: Panel,
    pub app_menu: AppMenu,
    pub taskbar: Taskbar,
    pub systray: SystemTray,

    // State
    pub needs_redraw: bool,
    pub last_mouse_buttons: u8,
    pub panel_window_id: Option<u32>,
    pub ui_runtime: Option<ui::UiRuntime>,
}

impl DesktopShell {
    pub fn new(screen_w: usize, screen_h: usize) -> Self {
        let panel = Panel::new(screen_w, screen_h);
        let app_menu = AppMenu::new(screen_w, screen_h, panel.height);
        let systray = SystemTray::new(
            panel.layout.systray_x,
            panel.layout.systray_w,
            screen_w,
        );

        DesktopShell {
            screen_w,
            screen_h,
            wallpaper: WallpaperManager::new(screen_w, screen_h),
            panel,
            app_menu,
            taskbar: Taskbar::new(),
            systray,
            needs_redraw: true,
            last_mouse_buttons: 0,
            panel_window_id: None,
            ui_runtime: None,
        }
    }

    fn build_mvp_widget_tree() -> Box<dyn Widget> {
        let mut root = Container::new(1, ContainerKind::Column);
        root.gap = 10;

        let title = Text::new(2, "Florynx UI Toolkit MVP");
        let input = Input::new(3, "Type here...");

        let mut actions = Container::new(4, ContainerKind::Row);
        actions.gap = 8;
        actions.push(Box::new(Button::new(5, "Launch")));
        actions.push(Box::new(Button::new(6, "Settings")));

        root.push(Box::new(title));
        root.push(Box::new(input));
        root.push(Box::new(actions));

        Box::new(root)
    }

    /// Handle click on the panel.
    pub fn on_panel_click(&mut self, x: usize, _y: usize) {
        let zone = self.panel.hit_zone(x);
        match zone {
            0 => {
                // App menu button
                self.app_menu.toggle();
                self.needs_redraw = true;
            }
            1 => {
                // Taskbar zone
                let offset = x.saturating_sub(self.panel.layout.taskbar_x);
                if let Some(win_id) = self.taskbar.entry_at(offset, self.panel.layout.taskbar_w) {
                    let _ = api::focus_window(win_id);
                    self.needs_redraw = true;
                }
            }
            2 => {
                // System tray click (toggle settings / calendar)
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    /// Handle click on the app menu.
    pub fn on_menu_click(&mut self, _x: usize, y: usize) -> Option<usize> {
        if !self.app_menu.visible { return None; }

        // Calculate which entry was clicked
        let item_h = 48;
        let header_h = 60;
        let rel_y = y.saturating_sub(self.app_menu.menu_y + header_h);
        let idx = rel_y / item_h;

        let apps = self.app_menu.filtered_apps();
        if idx < apps.len() {
            let icon_idx = apps[idx].icon_idx;
            self.app_menu.close();
            self.needs_redraw = true;
            return Some(icon_idx);
        }
        None
    }

    /// Register a new window on the taskbar.
    pub fn register_window(&mut self, win_id: u32, title: &str, icon_idx: usize) {
        self.taskbar.add(win_id, title, icon_idx);
        self.needs_redraw = true;
    }

    /// Unregister a closed window from the taskbar.
    pub fn unregister_window(&mut self, win_id: u32) {
        self.taskbar.remove(win_id);
        self.needs_redraw = true;
    }

    /// Set active window on taskbar.
    pub fn set_active_window(&mut self, win_id: u32) {
        self.taskbar.set_active(win_id);
        self.needs_redraw = true;
    }

    /// Update system clock.
    pub fn update_clock(&mut self, h: u8, m: u8, s: u8) {
        self.systray.update_clock(h, m, s);
        // Only redraw systray area, not full screen
    }

    /// Cycle wallpaper.
    pub fn next_wallpaper(&mut self) {
        self.wallpaper.next();
        self.needs_redraw = true;
    }

    /// Get the usable desktop area (screen minus panel).
    pub fn desktop_area(&self) -> (usize, usize, usize, usize) {
        match self.panel.position {
            super::panel::PanelPosition::Bottom => {
                (0, 0, self.screen_w, self.screen_h.saturating_sub(self.panel.height))
            }
            super::panel::PanelPosition::Top => {
                (0, self.panel.height, self.screen_w, self.screen_h.saturating_sub(self.panel.height))
            }
        }
    }

    /// Drain a bounded number of kernel GUI events and update shell state.
    /// Returns number of processed events.
    pub fn pump_kernel_events_once(&mut self, max_events: usize) -> usize {
        let mut processed = 0usize;

        while processed < max_events {
            let ev = match api::poll_event_v1() {
                Some(e) => e,
                None => break,
            };

            match ev {
                GuiEventV1::MouseState { win_id, x, y, buttons } => {
                    if let Some(panel_id) = self.panel_window_id {
                        if win_id != panel_id {
                            processed += 1;
                            continue;
                        }
                    }
                    let left_now = (buttons & 1) != 0;
                    let left_was = (self.last_mouse_buttons & 1) != 0;
                    if left_now && !left_was && self.panel.contains(x as usize, y as usize) {
                        self.on_panel_click(x as usize, y as usize);
                    }
                    self.last_mouse_buttons = buttons;
                    if let Some(runtime) = self.ui_runtime.as_mut() {
                        if win_id == runtime.window_id {
                            if let Some(event) = ui::UiRuntime::map_kernel_event(
                                GuiEventV1::MouseState { win_id, x, y, buttons },
                            ) {
                                let _ = runtime.handle_event(event);
                                self.needs_redraw = true;
                            }
                        }
                    }
                }
                GuiEventV1::KeyPress { win_id, code } => {
                    // Reserved for shell shortcuts in later iterations.
                    if let Some(runtime) = self.ui_runtime.as_mut() {
                        if win_id == runtime.window_id {
                            if let Some(event) = ui::UiRuntime::map_kernel_event(
                                GuiEventV1::KeyPress { win_id, code },
                            ) {
                                let _ = runtime.handle_event(event);
                                self.needs_redraw = true;
                            }
                        }
                    }
                }
                GuiEventV1::WindowCreated { win_id } => {
                    if Some(win_id) != self.panel_window_id {
                        self.register_window(win_id, "App", 0);
                    }
                }
                GuiEventV1::WindowDestroyed { win_id } => {
                    self.unregister_window(win_id);
                }
                GuiEventV1::WindowFocused { win_id } => {
                    self.set_active_window(win_id);
                }
                GuiEventV1::WindowResized { win_id, w: _, h: _ } => {
                    // Window resized in kernel — update taskbar if needed
                    let _ = win_id;
                    self.needs_redraw = true;
                }
            }

            processed += 1;
        }

        processed
    }

    /// Create and remember a userland panel window in the kernel compositor.
    pub fn register_panel_window(&mut self) -> Option<u32> {
        if let Some(id) = self.panel_window_id {
            return Some(id);
        }
        let panel_y = self.panel.y() as u32;
        let id = api::create_window(0, panel_y, self.screen_w as u32, self.panel.height as u32);
        if id < 0 {
            return None;
        }
        let win_id = id as u32;
        let _ = api::draw_rect(
            win_id,
            0,
            0,
            self.screen_w.min(HGUI_SCREEN_W as usize) as u32,
            self.panel.height as u32,
            0x1D222D,
        );
        let _ = api::draw_text(win_id, HGUI_PANEL_TITLE);
        let _ = api::invalidate(win_id);
        let root = Self::build_mvp_widget_tree();
        let mut runtime = ui::UiRuntime::new(
            win_id,
            root,
            ui::Size {
                w: self.screen_w as i32,
                h: self.panel.height as i32,
            },
        );
        runtime.layout();
        runtime.render();
        self.ui_runtime = Some(runtime);
        self.panel_window_id = Some(win_id);
        Some(win_id)
    }

    /// Destroy the userland panel window if registered.
    pub fn destroy_panel_window(&mut self) -> bool {
        let panel_id = match self.panel_window_id {
            Some(id) => id,
            None => return true,
        };
        let rc = api::destroy_window(panel_id);
        if rc >= 0 {
            self.panel_window_id = None;
            self.ui_runtime = None;
            true
        } else {
            false
        }
    }

    /// Frame tick for retained-mode userland UI.
    pub fn tick_ui_frame(&mut self, dt_ms: u32) {
        if let Some(runtime) = self.ui_runtime.as_mut() {
            runtime.tick_animations(dt_ms);
            runtime.layout();
            runtime.render();
            self.needs_redraw = false;
        }
    }
}
