// =============================================================================
// Florynx Userland — Raw Syscall Wrapper
// =============================================================================

#[inline(always)]
pub fn syscall3(nr: u64, arg1: u64, arg2: u64, arg3: u64) -> i64 {
    #[cfg(target_arch = "x86_64")]
    {
        let ret: i64;
        unsafe {
            core::arch::asm!(
                "int 0x80",
                in("rax") nr,
                in("rdi") arg1,
                in("rsi") arg2,
                in("rdx") arg3,
                lateout("rax") ret,
                options(nostack)
            );
        }
        ret
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        let _ = (nr, arg1, arg2, arg3);
        -38
    }
}

pub fn abi_info() -> Result<florynx_shared::syscall_abi::AbiInfoV1, i64> {
    let mut out = florynx_shared::syscall_abi::AbiInfoV1 {
        hdr: florynx_shared::syscall_abi::AbiHeader {
            size: 0,
            version: 0,
        },
        abi_major: 0,
        abi_minor: 0,
        user_stat_size: 0,
    };
    let rc = syscall3(
        florynx_shared::syscall_abi::SYS_ABI_INFO,
        (&mut out as *mut florynx_shared::syscall_abi::AbiInfoV1) as u64,
        core::mem::size_of::<florynx_shared::syscall_abi::AbiInfoV1>() as u64,
        0,
    );
    if rc < 0 {
        Err(rc)
    } else if out.hdr.version != florynx_shared::syscall_abi::ABI_V1
        || out.hdr.size != core::mem::size_of::<florynx_shared::syscall_abi::AbiInfoV1>() as u16
    {
        Err(florynx_shared::syscall_abi::E_INVAL)
    } else {
        Ok(out)
    }
}

pub fn debug_telemetry() -> Result<florynx_shared::syscall_abi::KernelTelemetryV1, i64> {
    let mut out = florynx_shared::syscall_abi::KernelTelemetryV1 {
        hdr: florynx_shared::syscall_abi::AbiHeader {
            size: 0,
            version: 0,
        },
        page_fault_total: 0,
        page_fault_user: 0,
        page_fault_kernel: 0,
        panic_count: 0,
    };
    let rc = syscall3(
        florynx_shared::syscall_abi::SYS_DEBUG_TELEMETRY,
        (&mut out as *mut florynx_shared::syscall_abi::KernelTelemetryV1) as u64,
        core::mem::size_of::<florynx_shared::syscall_abi::KernelTelemetryV1>() as u64,
        0,
    );
    if rc < 0 {
        Err(rc)
    } else if out.hdr.version != florynx_shared::syscall_abi::ABI_V1
        || out.hdr.size != core::mem::size_of::<florynx_shared::syscall_abi::KernelTelemetryV1>() as u16
    {
        Err(florynx_shared::syscall_abi::E_INVAL)
    } else {
        Ok(out)
    }
}

/// Probe usercopy protection by intentionally passing an invalid userspace pointer.
/// Expected result on a hardened kernel path is -EFAULT.
pub fn probe_efault() -> i64 {
    // SYS_WRITE(fd=1, buf_ptr=0, len=8) should fail pointer validation.
    syscall3(florynx_shared::syscall_abi::SYS_WRITE, 1, 0, 8)
}

