// =============================================================================
// Florynx Kernel — CPU Utilities
// =============================================================================
// CPU feature detection and control utilities.
// =============================================================================

/// Check if CPUID is available on this processor.
pub fn has_cpuid() -> bool {
    // On x86_64, CPUID is always available
    true
}

/// Enable SSE and FPU on this CPU.
///
/// Must be called before any SSE/FPU instruction is executed.
/// Sets:
///   CR0.EM  = 0  (no coprocessor emulation — use real FPU)
///   CR0.MP  = 1  (monitor coprocessor — trap WAIT/FWAIT when TS=1)
///   CR4.OSFXSR      = 1  (OS supports FXSAVE/FXRSTOR)
///   CR4.OSXMMEXCPT  = 1  (OS handles SIMD FP exceptions)
/// Then calls FNINIT to put the FPU in a known-good initial state.
pub fn enable_sse() {
    use x86_64::registers::control::{Cr0, Cr0Flags, Cr4, Cr4Flags};
    unsafe {
        let mut cr0 = Cr0::read();
        cr0.remove(Cr0Flags::EMULATE_COPROCESSOR);
        cr0.insert(Cr0Flags::MONITOR_COPROCESSOR);
        Cr0::write(cr0);

        let mut cr4 = Cr4::read();
        cr4.insert(Cr4Flags::OSFXSR);
        cr4.insert(Cr4Flags::OSXMMEXCPT_ENABLE);
        Cr4::write(cr4);

        core::arch::asm!("fninit", options(nomem, nostack, preserves_flags));
    }
    crate::serial_println!("[cpu] SSE/FPU enabled");
}

/// Set CR0.TS (Task Switched) — forces a #NM on next FPU/SSE use.
/// Called after a context switch to trigger lazy FPU save/restore.
#[inline]
pub fn set_task_switched() {
    use x86_64::registers::control::{Cr0, Cr0Flags};
    unsafe {
        let mut cr0 = Cr0::read();
        cr0.insert(Cr0Flags::TASK_SWITCHED);
        Cr0::write(cr0);
    }
}

/// Clear CR0.TS — allow FPU/SSE instructions without trapping.
/// Called from the #NM handler after FPU state has been restored.
#[inline]
pub fn clear_task_switched() {
    unsafe { core::arch::asm!("clts", options(nomem, nostack, preserves_flags)); }
}

/// Read the CPU vendor string from CPUID.
pub fn vendor_string() -> [u8; 12] {
    let mut vendor = [0u8; 12];
    unsafe {
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov [{0}], ebx",
            "mov [{0} + 4], edx",
            "mov [{0} + 8], ecx",
            "pop rbx",
            in(reg) vendor.as_mut_ptr(),
            inout("eax") 0u32 => _,
            out("ecx") _,
            out("edx") _,
        );
    }
    vendor
}

/// Log CPU information to the serial port.
pub fn log_cpu_info() {
    let vendor = vendor_string();
    if let Ok(vendor_str) = core::str::from_utf8(&vendor) {
        crate::serial_println!("[cpu] vendor: {}", vendor_str);
    }
}

/// Halt the CPU until the next interrupt.
#[inline]
pub fn halt() {
    x86_64::instructions::hlt();
}
