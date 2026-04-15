# Florynx OS Advanced Security Audit & Exploit Report

> [!WARNING]
> This document outlines critical, remotely and locally exploitable vulnerabilities within the current FlorynxOS kernel source. These vulnerabilities allow complete Ring 3 to Ring 0 privilege escalation, kernel memory extraction, and Denial-of-Service attacks.

## 🔴 PART 1 — ATTACK SURFACE MAPPING

The primary boundary between Userland (Ring 3) and Kernel (Ring 0) in FlorynxOS currently exposes the following attack surfaces:

1. **System Call Interface (`sys_*`)**: Exposes file system operations (`sys_read`, `sys_write`), process management (`sys_fork`, `sys_execve`), and inter-process polling loops. 
2. **GUI / Window Manager (`sys_gui_*`)**: Allows raw coordinate manipulations (`sys_gui_draw_rect`), text injection (`sys_gui_draw_text`), and unbound redraw invalidations.
3. **Memory Mapping & Access**: `copy_from_user` / `copy_to_user` are the primary bottlenecks mapping untrusted slices to kernel arrays.
4. **Hardware Drivers**: Interrupt-driven logic (PS/2 keyboard/mouse) processing unsolicited, potentially corrupted bytes.

*Most Exposed Components:* Syscall memory validation (`usermem.rs`) and memory layout management (`paging.rs`).

---

## 💀 PART 2 & 3 — USER → KERNEL ESCALATION & MEMORY CORRUPTION

### 🔴 CRITICAL: Time-Of-Check to Time-Of-Use (TOCTOU) in `usermem.rs`
The implementation of `usermem::validate_user_range` attempts to secure memory copies by performing a manual page-table walk prior to executing a raw `core::ptr::copy_nonoverlapping`.
```rust
pub fn copy_from_user(ptr: u64, len: u64) -> Result<Vec<u8>, i64> {
    validate_user_range(ptr, len, Access::Read)?; // <-- TIME OF CHECK
    // ...
    unsafe { core::ptr::copy_nonoverlapping(ptr as *const u8, out.as_mut_ptr(), len); } // <-- TIME OF USE
    // ...
}
```
**The Exploit:** In a multi-threaded user process, Thread A can pass a valid pointer to a syscall. Thread B concurrently issues an unmap command (or naturally faults) against that page. `copy_nonoverlapping` proceeds reading from an unmapped page. Since the kernel has no exception handling boundary (e.g., custom `#PF` catchers or `_copy_to_user` fault extables), a Page Fault in Ring 0 forces an immediate **Kernel Panic** (Denial of Service).

### 🔴 CRITICAL: Kernel Stack Memory Leak via Struct Padding
Both `UserStatV1` and `KernelTelemetryV1` contain explicit struct padding inserted by the compiler.
```rust
#[repr(C)]
pub struct UserStatV1 {
    pub hdr: AbiHeader, // 4 bytes
    // <-- COMPILER INSERTS 4 BYTES PADDING FOR ALIGNMENT -->
    pub inode: u64,     // 8 bytes
    // ...
}
```
**The Exploit:** When constructing these structs in `handlers.rs`, the padding bytes remain uninitialized, inheriting whatever residual data exists on the kernel stack (pointers, passwords, security keys). When the struct is cast to `[u8]` and copied to userland via `copy_to_user`, the kernel stack is effectively leaked piece by piece.

---

## ⚙️ PART 4 — SYSCALL HARDENING

### 🟡 MEDIUM: OOM & Bounds Integer Overflow (`sys_gui_draw_rect`)
The GUI syscall APIs take `packed_wh` integers.
```rust
let w = ((_packed_wh_color >> 48) & 0xFFFF) as usize;
let h = ((_packed_wh_color >> 32) & 0xFFFF) as usize;
```
If a malicious user submits `0xFFFF` for both width and height, the drawing engine iterates through extreme loops, starving the kernel scheduler. Although it does `clamp()` in `mark_dirty()`, the `Desktop` layer processes overlapping merges dynamically. Extreme overlap rendering leads to a **Soft-Lock (DoS)**.

---

## 🔌 PART 5 — DRIVER EXPLOITS

### 🟡 MEDIUM: Arbitrary Process Disruption via PS/2 Flooding
The `MOUSE` state machine relies on a 3-byte cycle. An attacker utilizing VM introspection/injection or malicious hardware mimicking a PS/2 component can flood the `status_port`. Since `wait_read()` blocks in busy loops for up to `100_000` cycles per byte inside an interrupt, it leads to system lockup, halting processes on `EAGAIN` wait states.

---

## 🧬 PART 6 — PROCESS ISOLATION BYPASS

### 🟢 MINOR: Uncontrolled Kernel Half Mapping
`paging.rs: create_user_page_table` statically copies all entries `0..512` from the active L4 table. Userland sees the entire kernel mapping structure. Process isolation functions solely because the `US` (User Accessible) bits are theoretically missing. This breaks Defense-In-Depth. A single bitflip or logic bug in `map_page_now` mapping a supervisor page incorrectly grants immediate Root Access.

---

## 💥 PART 7 — FAULT INJECTION (CRASH HANDLING WEAKNESSES)

The current OS immediately folds (Panics) if Ring 0 executes an invalid operation. If a syscall execution causes a division by zero or faults on a pointer dynamically passed, the OS crashes completely.

Fault Handling in `idt.rs` cannot distinguish between a legitimate kernel bug and a user-induced copy fault during valid syscall routing. This breaks process isolation robustness.

---

## 🧪 PART 8 — FUZZING STRATEGY

**Strategy Elements:**
1. **Syscall ABI Fuzzer:** Execute concurrently across forks. Target `sys_gui_draw_rect` with randomized packed 64-bit coordinates, width, and height. High risk of breaking the `WindowManager` rendering logic.
2. **IPC / Concurrency Fuzzer:** Fuzz `sys_wait` and `sys_waitpid`. Spawn zombies at extreme rates to test limits of the implicit Ring0 `sleep_current(1)` mechanics. Test for internal memory leaks across zombie states.
3. **Structured Payload Testing:** Rapidly invoke `sys_stat` grabbing millions of structs, parsing padding data, verifying for uninitialized entropy leak signatures.

---

## 🛡️ PART 9 — HARDENING RECOMMENDATIONS

### 1. Pointer Validation & Safe Memory Copy (Fixing the TOCTOU)
Rather than checking page tables, perform copies in an `unsafe` block relying on hardware traps.
*Conceptual Fix:* Implement an architecture-specific exception table.
```rust
// A much simpler fix if fully dynamic tables aren't ready:
pub fn safe_copy_from_user(ptr: u64, out: &mut [u8]) -> Result<(), i64> {
    // 1. Verify range (virt addr bounds)
    validate_range(ptr, out.len() as u64)?;
    // 2. Perform copy byte-by-byte catching faults, or rely on a wrapper 
    //    that temporarily alters the IDT `#PF` handler to gracefully return
    //    instead of panicking if the instruction pointer is inside this function.
}
```

### 2. Syscall Guard Template (Fixing the Stack Leak)
To stop padding leaks, explicitly enforce zero-padding on ABI interfaces:
```rust
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UserStatV1 {
    pub hdr: AbiHeader,
    pub _pad: u32,       // Explicit pad
    pub inode: u64,
    // ...
}
```
Always use `core::mem::zeroed()` or `Default::default()` before field assignment!

### 3. Rate-Limiting & Clipping for the GUI
Before passing bounds to the WindowManager:
```rust
// In sys_gui_draw_rect:
let max_w = crate::gui::desktop::DESKTOP.lock().map_or(0, |d| d.screen_w);
let safe_w = core::cmp::min(w, max_w);
// ... apply to `set_window_rect`
```

### 4. Guard Page Refinement
Ensure guard pages are universally applied at the base of every thread stack explicitly to prevent stack buffer overflow wrapping back into lower page allocations.
