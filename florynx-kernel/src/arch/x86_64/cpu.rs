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
