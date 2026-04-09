// =============================================================================
// Florynx Kernel — GUI Subsystem
// =============================================================================
// Lightweight windowing GUI built on the BGA framebuffer.
//   renderer  — drawing primitives (rect, rounded rect, text, shadow, cursor)
//   theme     — color palette and spacing constants
//   event     — input event types and geometry helpers
//   window    — draggable window component
//   dock      — macOS-style bottom dock
//   desktop   — compositor, window manager, main draw/event loop
//   console   — framebuffer text console for early boot output
// =============================================================================

pub mod renderer;
pub mod theme;
pub mod event;
pub mod animation;
pub mod window;
pub mod dock;
pub mod desktop;
pub mod event_bus;
pub mod console;
pub mod icons;
pub mod widgets;
pub mod text_editor;
