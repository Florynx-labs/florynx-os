// =============================================================================
// Florynx Kernel — APIC Driver (stub)
// =============================================================================
// Placeholder for the Advanced Programmable Interrupt Controller (APIC).
// Will be used when upgrading from legacy PIC to APIC/x2APIC for SMP support.
// =============================================================================

/// Check if the local APIC is available via CPUID.
pub fn is_available() -> bool {
    // Stub: APIC detection via CPUID leaf 1, EDX bit 9
    false
}

/// Initialize the local APIC (stub).
pub fn init() {
    crate::serial_println!("[apic] stub — using legacy PIC");
}
