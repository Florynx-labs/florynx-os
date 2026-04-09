// =============================================================================
// Florynx Userland — System Monitor (KSysGuard-Style)
// =============================================================================

/// System monitor state.
pub struct SystemMonitor {
    pub cpu_usage: u8,
    pub mem_total_kb: u32,
    pub mem_used_kb: u32,
    pub task_count: u16,
    pub uptime_secs: u64,
    pub page_fault_total: u64,
    pub page_fault_user: u64,
    pub page_fault_kernel: u64,
    pub panic_count: u64,
    pub abi_major: u16,
    pub abi_minor: u16,
    pub last_diag_rc: i64,
}

impl SystemMonitor {
    pub fn new() -> Self {
        SystemMonitor {
            cpu_usage: 0,
            mem_total_kb: 16384,
            mem_used_kb: 0,
            task_count: 0,
            uptime_secs: 0,
            page_fault_total: 0,
            page_fault_user: 0,
            page_fault_kernel: 0,
            panic_count: 0,
            abi_major: 0,
            abi_minor: 0,
            last_diag_rc: 0,
        }
    }

    /// Refresh monitor diagnostics from kernel telemetry syscalls.
    pub fn refresh_diagnostics(&mut self) {
        if let Ok(abi) = crate::syscall::abi_info() {
            self.abi_major = abi.abi_major;
            self.abi_minor = abi.abi_minor;
        }

        if let Ok(t) = crate::syscall::debug_telemetry() {
            self.page_fault_total = t.page_fault_total;
            self.page_fault_user = t.page_fault_user;
            self.page_fault_kernel = t.page_fault_kernel;
            self.panic_count = t.panic_count;
        }
    }

    /// Run a safe EFAULT probe and return the syscall return code.
    pub fn run_efault_probe(&mut self) -> i64 {
        let rc = crate::syscall::probe_efault();
        self.last_diag_rc = rc;
        rc
    }
}
