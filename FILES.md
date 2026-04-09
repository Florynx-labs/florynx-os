# FlorynxOS P0 Implementation Notes

This file explains what has been implemented in the current P0 hardening cycle and why.

## Scope

Primary focus so far:

- Ring 3 isolation groundwork
- User memory protection for syscalls
- Process lifecycle hardening
- ABI stabilization primitives
- Crash safety + telemetry visibility

## What Was Implemented

### 1) User Memory Protection

- Added hardened syscall copy helpers:
  - `copy_from_user`
  - `copy_to_user`
- Added pointer/range validation:
  - null checks
  - overflow checks
  - canonical address checks
  - higher-half kernel address rejection
- Added page-table permission validation for user pointers:
  - `PRESENT`
  - `USER_ACCESSIBLE`
  - `WRITABLE` (for write paths)

Result:

- invalid user pointers now fail with `-EFAULT` instead of unsafe raw access patterns.

### 2) Kernel/User Mapping Isolation

- Added boot-time mapping audit to ensure kernel mappings are supervisor-only.
- Added helper for kernel page mapping policy:
  - kernel mappings explicitly keep `USER_ACCESSIBLE` off.

Result:

- catches isolation regressions at boot.

### 3) Page Fault Containment

- Page fault handler now distinguishes user faults vs kernel faults.
- User-mode fault path:
  - logs event
  - terminates offending task path (zombie transition path)
  - continues scheduling
- Kernel fault path:
  - retains panic behavior with diagnostics.

Result:

- user crashes are contained; kernel survives user faults.

### 4) Process Lifecycle Progress

- Migrated task state flow toward:
  - `Ready`
  - `Running`
  - `Sleeping`
  - `Zombie`
- Added lifecycle syscalls:
  - `wait`
  - `kill`
- `sleep` converted to scheduler-managed sleeping (non-busy path).
- Added zombie reap flow with cleanup hooks.

Result:

- more realistic lifecycle behavior and less scheduler-blocking behavior.

### 5) Resource Cleanup on Reap

- Added task-owned file descriptor tracking in VFS.
- Reap path now closes task-owned FDs automatically.
- Added centralized cleanup hook for future expansion to memory/frame/handle reclaim.

Result:

- concrete cleanup now occurs on reap (FDs), and cleanup path is extensible.

### 6) ABI Stabilization

- Added `SYS_ABI_INFO`.
- Added shared ABI header model:
  - `AbiHeader { size, version }`
- Added shared versioned structs:
  - `AbiInfoV1`
  - `UserStatV1`
- Kernel and userland now use shared ABI struct definitions and validate header compatibility.

Result:

- safer kernel/userland compatibility and less ABI drift risk.

### 7) Crash Safety Policy + Telemetry

- Added panic policy control:
  - `Halt`
  - `Reboot`
- Added panic counter telemetry.
- Added page fault counters:
  - total
  - user
  - kernel
- Added debug telemetry syscall:
  - `SYS_DEBUG_TELEMETRY`
  - returns `KernelTelemetryV1`

Result:

- runtime crash/fault visibility for verification and regression checks.

### 8) Userland Diagnostics Hooks

- Added userland wrappers to query:
  - ABI info
  - kernel telemetry
- Added safe EFAULT probe helper to verify usercopy hardening path.
- Extended monitor app state with diagnostic fields and refresh/probe methods.

Result:

- easy verification loop from userland without kernel rebuild per check.

## Why These Changes

These changes are designed to reduce the highest-risk failure classes first:

- kernel memory exposure from bad user pointers
- full-system panic on user faults
- missing process cleanup
- ABI mismatch bugs between kernel and userland

All work was done incrementally with compile checks to keep boot risk low.

## Current Status

- P0 is substantially advanced and testable.
- Verification interfaces are present.
- Remaining hardening includes deeper memory/frame reclaim and extended runtime tests.

## Next Recommended Steps

1. Add full address-space frame reclamation on zombie reap.
2. Run fault-injection matrix and record expected telemetry deltas.
3. Stabilize wait/kill semantics under stress (multiple children and rapid exits).
4. Then begin P1 persistent filesystem foundation.
