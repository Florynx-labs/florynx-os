# Florynx-OS v0.4.5 "Sentinel"

**Release Date**: April 9, 2026  
**Codename**: "Sentinel"  
**Focus**: P0 hardening (isolation, lifecycle, ABI, crash safety)

---

## Highlights in v0.4.5

### Kernel/User Isolation Hardening
- User memory access now goes through hardened copy helpers (`copy_from_user`, `copy_to_user`).
- Pointer validation now includes canonical/range checks and user-space guardrails.
- Page-table permission checks added for syscall user buffers.

### Process Lifecycle Progress
- Task state model moved toward `Ready/Running/Sleeping/Zombie`.
- Added lifecycle syscalls: `wait` and `kill`.
- Added zombie reaping with first cleanup hooks.

### ABI Stabilization
- Added `SYS_ABI_INFO` with versioned shared ABI struct headers (`size`, `version`).
- Shared structs standardized in `shared/` for kernel-userland contract safety.

### Crash Safety + Diagnostics
- User page faults are contained (task termination path) instead of unconditional kernel panic.
- Panic policy added (`halt` or controlled reboot path).
- Added fault/panic telemetry and `SYS_DEBUG_TELEMETRY`.

### Userland Diagnostics
- Userland wrappers for ABI and telemetry probing.
- Added safe EFAULT probe path for verification.

---

## Build & Run

```bash
cd florynx-kernel
cargo +nightly bootimage
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-florynx/debug/bootimage-florynx-kernel.bin \
  -serial stdio -m 128
```

---

## Notes

- This release prioritizes correctness and hardening over feature breadth.
- Recommended next milestone: P1 persistent filesystem + storage-backed VFS path.
