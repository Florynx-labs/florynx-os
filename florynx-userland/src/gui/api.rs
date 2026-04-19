// =============================================================================
// Florynx Userland — GUI Syscall API
// =============================================================================

use florynx_shared::syscall_abi;

use crate::syscall::syscall3;

#[inline]
fn pack_u32_pair(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | (lo as u64)
}

pub fn create_window(x: u32, y: u32, w: u32, h: u32) -> i64 {
    syscall3(
        syscall_abi::SYS_GUI_CREATE_WINDOW,
        x as u64,
        y as u64,
        pack_u32_pair(w, h),
    )
}

#[repr(C)]
pub struct GuiBlitArgs {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub buffer_ptr: u64,
}

pub fn draw_rect(win_id: u32, x: u32, y: u32, w: u32, h: u32, color_rgb: u32) -> i64 {
    let packed_xy = pack_u32_pair(x, y);
    let packed_wh_color = (((w as u64) & 0xFFFF) << 48)
        | (((h as u64) & 0xFFFF) << 32)
        | (color_rgb as u64);
    let _ = packed_wh_color;
    syscall3(
        syscall_abi::SYS_GUI_DRAW_RECT,
        win_id as u64,
        packed_xy,
        packed_wh_color,
    )
}

pub fn blit_buffer(win_id: u32, x: u32, y: u32, w: u32, h: u32, buffer: &[u8]) -> i64 {
    let args = GuiBlitArgs {
        x,
        y,
        w,
        h,
        buffer_ptr: buffer.as_ptr() as u64,
    };
    syscall3(
        syscall_abi::SYS_GUI_BLIT_BUFFER,
        win_id as u64,
        &args as *const GuiBlitArgs as u64,
        0,
    )
}

pub fn draw_text(win_id: u32, text: &str) -> i64 {
    syscall3(
        syscall_abi::SYS_GUI_DRAW_TEXT,
        win_id as u64,
        text.as_ptr() as u64,
        text.len() as u64,
    )
}

pub fn poll_event(event_out_ptr: u64) -> i64 {
    syscall3(syscall_abi::SYS_GUI_POLL_EVENT, event_out_ptr, 0, 0)
}

pub fn invalidate(win_id: u32) -> i64 {
    syscall3(syscall_abi::SYS_GUI_INVALIDATE, win_id as u64, 0, 0)
}

pub fn focus_window(win_id: u32) -> i64 {
    syscall3(syscall_abi::SYS_GUI_FOCUS_WINDOW, win_id as u64, 0, 0)
}

pub fn destroy_window(win_id: u32) -> i64 {
    syscall3(syscall_abi::SYS_GUI_DESTROY_WINDOW, win_id as u64, 0, 0)
}

/// Convenience helper for smoke-testing userland->kernel GUI bridge.
pub fn create_demo_window() -> i64 {
    let win_id = create_window(140, 120, 520, 300);
    if win_id < 0 {
        return win_id;
    }
    let _ = draw_rect(win_id as u32, 20, 30, 120, 48, 0x2AA198);
    let _ = draw_text(win_id as u32, "Hello from florynx-userland via SYS_GUI_DRAW_TEXT");
    let _ = invalidate(win_id as u32);
    win_id
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiEventV1 {
    MouseState { win_id: u32, x: u32, y: u32, buttons: u8 },
    KeyPress { win_id: u32, code: u16 },
    WindowCreated { win_id: u32 },
    WindowDestroyed { win_id: u32 },
    WindowFocused { win_id: u32 },
    WindowResized { win_id: u32, w: u16, h: u16 },
}

pub fn poll_event_v1() -> Option<GuiEventV1> {
    let mut raw: u64 = 0;
    let rc = poll_event((&mut raw as *mut u64) as u64);
    if rc <= 0 {
        return None;
    }
    let ty = ((raw >> 56) & 0xFF) as u8;
    match ty {
        1 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            let buttons = ((raw >> 32) & 0xFF) as u8;
            let x = ((raw >> 16) & 0xFFFF) as u32;
            let y = (raw & 0xFFFF) as u32;
            Some(GuiEventV1::MouseState { win_id, x, y, buttons })
        }
        2 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            let code = ((raw >> 24) & 0xFFFF) as u16;
            Some(GuiEventV1::KeyPress { win_id, code })
        }
        3 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            Some(GuiEventV1::WindowCreated { win_id })
        }
        4 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            Some(GuiEventV1::WindowDestroyed { win_id })
        }
        5 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            Some(GuiEventV1::WindowFocused { win_id })
        }
        6 => {
            let win_id = ((raw >> 40) & 0xFFFF) as u32;
            let w = ((raw >> 16) & 0xFFFF) as u16;
            let h = (raw & 0xFFFF) as u16;
            Some(GuiEventV1::WindowResized { win_id, w, h })
        }
        _ => None,
    }
}

