// =============================================================================
// Florynx Kernel — HGUI Link (Kernel <-> Userland)
// =============================================================================
// Clean handoff module that prepares user-owned shell surfaces while waiting
// for full Ring3 userland launcher integration.
// =============================================================================

pub fn launch_core() {
    crate::gui::console::disable();
    crate::gui::desktop::init_empty();
    crate::gui::desktop::draw();
    crate::gui::renderer::update_cursor(400, 300);
    crate::serial_println!("[hgui] core launched (userland-owned UI)");
}

