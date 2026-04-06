// =============================================================================
// Florynx Kernel — Low-Level x86_64 Assembly Utilities
// =============================================================================
// Provides safe (where possible) Rust wrappers around inline assembly for:
//
//   Part 1: Context Switching    — switch_to(), init_task_stack()
//   Part 2: Interrupt Control    — enable/disable_interrupts(), without_interrupts()
//   Part 3: I/O Port Access      — outb(), inb(), outw(), inw(), io_wait()
//   Part 4: Descriptor Tables    — load_gdt(), load_idt()
//   Part 5: Debug & Diagnostics  — read_rsp(), read_rflags(), read_cr3()
//
// All functions follow the System V AMD64 ABI.
// All unsafe blocks document their safety invariants.
// =============================================================================

use core::arch::asm;

// =============================================================================
// PART 1 — CONTEXT SWITCH (Multitasking Core)
// =============================================================================
//
// The context switch saves/restores only callee-saved registers (per System V
// AMD64 ABI: RBX, RBP, R12–R15). Caller-saved registers are already preserved
// by the Rust compiler's calling convention.
//
// Stack frame layout after save (low address = top of stack):
//
//   RSP → [ R15 ]   ← pushed last, popped first
//         [ R14 ]
//         [ R13 ]
//         [ R12 ]
//         [ RBX ]
//         [ RBP ]   ← pushed first, popped last
//         [ RIP ]   ← return address (from `call switch_to`)
//
// Total: 7 × 8 = 56 bytes. Since `call` pushes RIP (making RSP = 16n-8),
// and we push 6 more (48 bytes), final RSP = 16n - 56 = 16(n-4) + 8.
// The matching pop sequence on the other stack restores alignment symmetrically.
// =============================================================================

/// Perform a cooperative context switch between two kernel tasks.
///
/// # What it does
/// 1. Pushes callee-saved registers (RBX, RBP, R12–R15) onto current stack
/// 2. Saves current RSP into `*prev_stack_ptr`
/// 3. Loads `next_stack` into RSP (switching to new task's stack)
/// 4. Pops callee-saved registers from new stack
/// 5. Executes `ret`, which pops RIP → resumes the new task
///
/// # Why it is safe (when preconditions are met)
/// - Preserves all callee-saved registers per System V AMD64 ABI
/// - Caller-saved registers are handled by the Rust compiler
/// - Push/pop sequence is symmetric — no stack corruption
/// - No memory accesses beyond the two stacks
///
/// # Safety
/// - `prev_stack_ptr` must point to a valid, writable `u64` in kernel memory
/// - `next_stack` must be a valid RSP value from a prior `switch_to` save,
///   or from `init_task_stack()` for a brand-new task
/// - Both stacks must be in mapped, writable kernel memory
/// - Interrupts SHOULD be disabled during the switch to prevent re-entrant
///   scheduling from a timer IRQ
///
/// # Possible failure cases
/// - `next_stack` pointing to unmapped memory → page fault (kernel panic)
/// - Corrupted switch frame on next_stack → register corruption, crash
/// - Interrupts enabled during switch → re-entrant timer IRQ could
///   trigger another switch_to before this one completes
///
/// # QEMU testing strategy
/// 1. Create two tasks with separate 4 KiB stacks
/// 2. Initialize each with `init_task_stack(stack_top, entry_fn as u64)`
/// 3. Call `switch_to(task_b_sp, &mut task_a_sp)` from task A
/// 4. Verify via serial that task B executes, then switches back
/// 5. Expected: alternating "task A" / "task B" messages on serial
///
/// # ABI
/// Uses `extern "C"` — first arg (next_stack) in RDI, second (prev_stack_ptr) in RSI.
#[unsafe(naked)]
pub unsafe extern "C" fn switch_to(next_stack: u64, prev_stack_ptr: *mut u64) {
    // Naked function: we control the entire prologue and epilogue.
    // No Rust-generated code before or after this asm block.
    core::arch::naked_asm!(
        // ---- Save current task's callee-saved registers ----
        "push rbp",            // save base pointer
        "push rbx",            // save general-purpose callee-saved
        "push r12",            // save r12
        "push r13",            // save r13
        "push r14",            // save r14
        "push r15",            // save r15

        // ---- Store current RSP into *prev_stack_ptr ----
        // RSI = prev_stack_ptr (second argument, System V AMD64)
        "mov [rsi], rsp",

        // ---- Load next task's stack pointer ----
        // RDI = next_stack (first argument, System V AMD64)
        "mov rsp, rdi",

        // ---- Restore next task's callee-saved registers ----
        // Pop in reverse order of the save above
        "pop r15",             // restore r15
        "pop r14",             // restore r14
        "pop r13",             // restore r13
        "pop r12",             // restore r12
        "pop rbx",             // restore rbx
        "pop rbp",             // restore rbp

        // ---- Resume next task ----
        // `ret` pops RIP from the stack — either:
        //   (a) the return address from a previous switch_to call, or
        //   (b) the entry_point set up by init_task_stack()
        "ret",
    );
}

/// Initialize a new task's kernel stack for its first context switch.
///
/// Sets up the stack so that when `switch_to()` restores from it, the task
/// begins executing at `entry_point` with zeroed callee-saved registers.
///
/// # Stack layout after initialization (high → low address):
/// ```text
///   stack_top →  (unused — stack grows downward)
///                [ entry_point ]   ← `ret` will pop this as RIP
///                [ 0 = RBP     ]   ← `pop rbp` reads this
///                [ 0 = RBX     ]   ← `pop rbx` reads this
///                [ 0 = R12     ]   ← `pop r12` reads this
///                [ 0 = R13     ]   ← `pop r13` reads this
///                [ 0 = R14     ]   ← `pop r14` reads this
///   returned → [ 0 = R15     ]   ← `pop r15` reads this (RSP points here)
/// ```
///
/// # Returns
/// The stack pointer value to pass as `next_stack` to `switch_to()`.
///
/// # Safety
/// - `stack_top` must point to the END (highest address) of a valid,
///   mapped, writable memory region of at least 56 bytes
/// - `entry_point` must be a valid function address in kernel space
/// - The function at `entry_point` must never return (or must call
///   switch_to / schedule to yield)
pub unsafe fn init_task_stack(stack_top: *mut u64, entry_point: u64) -> u64 {
    let mut sp = stack_top;

    // Push the entry point — `ret` in switch_to will pop this as RIP
    sp = sp.sub(1);
    sp.write(entry_point);

    // Push zeroed callee-saved registers in the order that switch_to pops them.
    // switch_to pops: r15, r14, r13, r12, rbx, rbp (bottom to top of stack)
    // So we write:    rbp, rbx, r12, r13, r14, r15 (top to bottom in memory)
    sp = sp.sub(1); sp.write(0); // RBP
    sp = sp.sub(1); sp.write(0); // RBX
    sp = sp.sub(1); sp.write(0); // R12
    sp = sp.sub(1); sp.write(0); // R13
    sp = sp.sub(1); sp.write(0); // R14
    sp = sp.sub(1); sp.write(0); // R15

    // Return the stack pointer — switch_to will load this into RSP
    sp as u64
}


// =============================================================================
// PART 2 — INTERRUPT CONTROL
// =============================================================================
//
// These wrappers provide direct sti/cli access without depending on the
// x86_64 crate. They are safe functions because sti/cli only modify the
// IF flag in RFLAGS — they cannot corrupt memory or registers.
//
// However, MISUSING them can cause system instability:
//   - enable_interrupts() before IDT setup → triple fault
//   - disable_interrupts() for too long → timer drift, missed IRQs
// =============================================================================

/// Enable maskable hardware interrupts (STI).
///
/// # What it does
/// Sets the Interrupt Flag (IF) in RFLAGS. The CPU will begin responding
/// to maskable hardware interrupts (IRQs from PIC/APIC) after the NEXT
/// instruction following STI (one-instruction delay by hardware design).
///
/// # When to use
/// - After all interrupt handlers and hardware are configured
/// - After a critical section that temporarily disabled interrupts
/// - Before entering HLT in the main kernel loop
///
/// # When NOT to use
/// - During early boot before IDT/PIC are initialized (→ triple fault)
/// - Inside interrupt handlers (auto-disabled on entry, use IRET to restore)
/// - During context switch (risk of re-entrant scheduling)
///
/// # Why this is a safe function
/// STI only sets a CPU flag — it cannot corrupt memory, registers, or stack.
/// The danger is LOGICAL (enabling interrupts at the wrong time), not UB.
///
/// # QEMU testing
/// 1. Call `disable_interrupts()` then `enable_interrupts()`
/// 2. Verify timer ticks resume (PIT serial output continues)
/// 3. Expected: PIT ticks stop during disabled, resume after enable
#[inline(always)]
pub fn enable_interrupts() {
    // SAFETY: STI only modifies the IF flag in RFLAGS.
    // No memory writes, no register clobbers, no stack changes.
    unsafe {
        asm!(
            "sti",
            options(nomem, nostack)
        );
    }
}

/// Disable maskable hardware interrupts (CLI).
///
/// # What it does
/// Clears the Interrupt Flag (IF) in RFLAGS. The CPU will ignore all
/// maskable hardware interrupts until IF is set again (via STI or IRET).
///
/// # When to use
/// - Before modifying shared data accessed by IRQ handlers
/// - During context switch operations
/// - Before critical hardware programming sequences (e.g., PIC remapping)
///
/// # When NOT to use
/// - For extended periods → causes timer drift and missed input
/// - As a substitute for proper locking (use spinlocks for multi-core)
///
/// # Why this is a safe function
/// CLI only clears a CPU flag. Same rationale as enable_interrupts().
///
/// # QEMU testing
/// Same as enable_interrupts — verify interrupts stop during disabled period.
#[inline(always)]
pub fn disable_interrupts() {
    // SAFETY: CLI only modifies the IF flag in RFLAGS.
    unsafe {
        asm!(
            "cli",
            options(nomem, nostack)
        );
    }
}

/// Check whether maskable interrupts are currently enabled.
///
/// Reads the IF bit (bit 9) from the RFLAGS register.
/// Returns `true` if interrupts are enabled, `false` otherwise.
///
/// # Why this is safe
/// PUSHFQ/POP only read RFLAGS onto the stack and into a register.
/// No side effects whatsoever.
#[inline(always)]
pub fn interrupts_enabled() -> bool {
    let rflags: u64;
    // SAFETY: pushfq pushes RFLAGS, pop loads it into a register.
    // No side effects, no memory writes beyond the stack push/pop.
    unsafe {
        asm!(
            "pushfq",          // push RFLAGS onto the stack
            "pop {}",          // pop into output register
            out(reg) rflags,
            options(nomem)     // no memory operands (stack ops are implicit)
        );
    }
    // IF = bit 9 of RFLAGS
    (rflags & (1 << 9)) != 0
}

/// Execute a closure with interrupts disabled, then restore previous IF state.
///
/// This is the PREFERRED way to protect critical sections. It correctly
/// handles nested calls: if interrupts were already disabled, it won't
/// spuriously re-enable them afterward.
///
/// # Example
/// ```no_run
/// let result = without_interrupts(|| {
///     // Critical section — no IRQs can fire here
///     shared_data.modify();
///     42
/// });
/// ```
///
/// # Why this is safe
/// Composes safe primitives (interrupts_enabled, disable, enable).
/// Restores the exact IF state that existed before the call.
#[inline]
pub fn without_interrupts<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let was_enabled = interrupts_enabled();
    if was_enabled {
        disable_interrupts();
    }

    let result = f();

    if was_enabled {
        enable_interrupts();
    }
    result
}


// =============================================================================
// PART 3 — I/O PORT ACCESS (Drivers / Hardware)
// =============================================================================
//
// x86 uses a separate 16-bit I/O address space (ports 0x0000–0xFFFF) for
// communicating with hardware devices. The IN and OUT instructions transfer
// data between a CPU register and an I/O port.
//
// These functions are unsafe because writing to arbitrary ports can
// reconfigure hardware in destructive ways (e.g., corrupting PIC state,
// resetting disk controllers).
// =============================================================================

/// Write a byte to an I/O port.
///
/// # What it does
/// Sends `data` to the hardware device mapped at I/O port `port` using
/// the x86 `OUT` instruction (8-bit variant: `out dx, al`).
///
/// # Why it is unsafe
/// Writing to an I/O port has REAL hardware side effects:
/// - Port 0x20: sends EOI to PIC (ends interrupt handling)
/// - Port 0x60: sends command to PS/2 controller
/// - Port 0x3F8: writes to COM1 serial port
/// - Port 0xCF8/0xCFC: PCI configuration access
/// Writing the wrong value to the wrong port can hang or crash the system.
///
/// # Possible failure cases
/// - Writing to a port that doesn't exist → usually ignored (no fault)
/// - Writing wrong values to PIC/PIT/keyboard → system malfunction
/// - Some legacy devices need an io_wait() delay after writes
///
/// # QEMU testing
/// ```no_run
/// // Write 'H' to COM1 serial port
/// unsafe { outb(0x3F8, b'H'); }
/// // Expected: 'H' appears in QEMU serial output
/// ```
#[inline(always)]
pub unsafe fn outb(port: u16, data: u8) {
    asm!(
        "out dx, al",          // OUT port[DX], data[AL]
        in("dx") port,         // port number in DX
        in("al") data,         // byte to write in AL
        options(nomem, nostack, preserves_flags)
    );
}

/// Read a byte from an I/O port.
///
/// # What it does
/// Reads a single byte from the hardware device mapped at I/O port `port`
/// using the x86 `IN` instruction (8-bit variant: `in al, dx`).
///
/// # Why it is unsafe
/// Reading from an I/O port can have side effects:
/// - Port 0x60: reading clears the keyboard scancode buffer
/// - Port 0x3F8: reading clears the serial receive buffer
/// - Some status ports auto-clear flags on read
///
/// # Possible failure cases
/// - Reading a non-existent port → returns 0xFF on most hardware
/// - Reading a port with side effects without proper sequencing → lost data
///
/// # QEMU testing
/// ```no_run
/// // Read COM1 line status register
/// let status = unsafe { inb(0x3FD) };
/// // Expected: bit 5 set = transmit buffer empty
/// ```
#[inline(always)]
pub unsafe fn inb(port: u16) -> u8 {
    let data: u8;
    asm!(
        "in al, dx",           // IN data[AL], port[DX]
        in("dx") port,         // port number in DX
        out("al") data,        // byte read into AL
        options(nomem, nostack, preserves_flags)
    );
    data
}

/// Write a 16-bit word to an I/O port.
///
/// Same as `outb` but transfers 16 bits. Used by devices like the PIT
/// (port 0x40) and PCI configuration (port 0xCF8).
///
/// # Safety
/// Same as `outb` — writing to arbitrary ports can corrupt hardware state.
#[inline(always)]
pub unsafe fn outw(port: u16, data: u16) {
    asm!(
        "out dx, ax",          // OUT port[DX], data[AX] (16-bit)
        in("dx") port,
        in("ax") data,
        options(nomem, nostack, preserves_flags)
    );
}

/// Read a 16-bit word from an I/O port.
///
/// Same as `inb` but transfers 16 bits.
///
/// # Safety
/// Same as `inb` — reading from some ports has side effects.
#[inline(always)]
pub unsafe fn inw(port: u16) -> u16 {
    let data: u16;
    asm!(
        "in ax, dx",           // IN data[AX], port[DX] (16-bit)
        in("dx") port,
        out("ax") data,
        options(nomem, nostack, preserves_flags)
    );
    data
}

/// Write a 32-bit doubleword to an I/O port.
///
/// Used by PCI configuration space access (port 0xCF8 for address,
/// port 0xCFC for data).
///
/// # Safety
/// Same as `outb`.
#[inline(always)]
pub unsafe fn outl(port: u16, data: u32) {
    asm!(
        "out dx, eax",         // OUT port[DX], data[EAX] (32-bit)
        in("dx") port,
        in("eax") data,
        options(nomem, nostack, preserves_flags)
    );
}

/// Read a 32-bit doubleword from an I/O port.
///
/// # Safety
/// Same as `inb`.
#[inline(always)]
pub unsafe fn inl(port: u16) -> u32 {
    let data: u32;
    asm!(
        "in eax, dx",          // IN data[EAX], port[DX] (32-bit)
        in("dx") port,
        out("eax") data,
        options(nomem, nostack, preserves_flags)
    );
    data
}

/// Small I/O delay for legacy hardware timing requirements.
///
/// # What it does
/// Writes a zero byte to port 0x80 (POST diagnostic port). This port is
/// universally safe to write on x86 systems and introduces a ~1 µs delay
/// that some legacy devices (8259 PIC, 8254 PIT, PS/2 controller) need
/// between consecutive I/O operations.
///
/// # Why this is safe (as a function)
/// Port 0x80 is a well-known diagnostic port. Writing to it has no
/// harmful side effects — it's used specifically as a timing mechanism.
/// However, the raw port write is still technically unsafe.
///
/// # QEMU testing
/// Insert between PIC/PIT configuration writes and verify no timing issues.
#[inline(always)]
pub fn io_wait() {
    // SAFETY: Port 0x80 is the POST diagnostic port.
    // Writing 0 to it is a standard, universally-safe I/O delay technique
    // used by Linux, SeaBIOS, and virtually all x86 operating systems.
    unsafe {
        asm!(
            "out 0x80, al",    // write to POST diagnostic port
            in("al") 0u8,     // value doesn't matter
            options(nomem, nostack, preserves_flags)
        );
    }
}


// =============================================================================
// PART 4 — DESCRIPTOR TABLE LOADING (GDT / IDT)
// =============================================================================
//
// The GDT defines memory segments and privilege levels. The IDT defines
// interrupt/exception handlers. Both are loaded via special instructions
// (LGDT/LIDT) that take a 10-byte pointer structure.
//
// NOTE: FlorynxOS currently loads GDT/IDT via the `x86_64` crate in
// gdt::init() and idt::init(). These raw primitives are provided for:
//   - Future custom GDT management (Ring 3 transitions)
//   - Reducing external crate dependency
//   - Educational clarity
// =============================================================================

/// Descriptor table pointer — the 10-byte structure expected by LGDT/LIDT.
///
/// Layout (as per Intel SDM Vol. 3A, Section 3.5.1):
/// - Bytes 0–1: Limit (size of table minus 1, in bytes)
/// - Bytes 2–9: Base address (linear address of byte 0 of the table)
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct DescriptorTablePointer {
    /// Size of the descriptor table minus 1 (max 65535 → 8192 entries).
    pub limit: u16,
    /// Linear (virtual) base address of the descriptor table.
    pub base: u64,
}

/// Load the Global Descriptor Table register (LGDT).
///
/// # What it does
/// Points the CPU's GDTR at the table described by `gdt_ptr`. After this,
/// segment selectors (CS, DS, SS, etc.) reference entries in the new GDT.
///
/// # Why it is safe (when preconditions are met)
/// LGDT itself just loads the GDTR register — it doesn't immediately change
/// execution flow. The risk comes from loading an INVALID GDT, which causes
/// a fault on the next segment register load or interrupt.
///
/// # Safety
/// - `gdt_ptr` must point to a valid, properly initialized `DescriptorTablePointer`
/// - The GDT at `base` must contain valid segment descriptors
/// - The GDT memory must remain mapped and valid for as long as it is active
/// - After LGDT, segment registers (CS, DS, SS) must be reloaded to use the
///   new descriptors. CS requires a far jump or far return.
///
/// # Possible failure cases
/// - Invalid base → triple fault on next segment register reload
/// - Limit too small → #GP when accessing descriptors beyond the limit
/// - Corrupt descriptors → #GP or #TS faults
///
/// # QEMU testing
/// 1. Construct a GDT with kernel code/data/TSS entries
/// 2. Call `load_gdt(&ptr)` followed by segment register reloads
/// 3. Expected: kernel continues executing, serial output uninterrupted
/// 4. Failure mode: triple fault → QEMU resets (visible in serial log)
#[inline]
pub unsafe fn load_gdt(gdt_ptr: &DescriptorTablePointer) {
    asm!(
        "lgdt [{}]",           // LGDT — load GDT register from memory
        in(reg) gdt_ptr,       // pointer to the 10-byte descriptor
        options(readonly, nostack, preserves_flags)
    );
}

/// Load the Interrupt Descriptor Table register (LIDT).
///
/// Same structure and preconditions as `load_gdt` but for the IDT.
///
/// # Safety
/// - `idt_ptr` must point to a valid `DescriptorTablePointer`
/// - The IDT at `base` must contain valid gate descriptors
/// - The IDT must remain in memory for as long as interrupts are active
///
/// # Possible failure cases
/// - Invalid IDT → #GP on the next interrupt (likely triple fault)
/// - Missing handler entries → #GP when that specific interrupt fires
#[inline]
pub unsafe fn load_idt(idt_ptr: &DescriptorTablePointer) {
    asm!(
        "lidt [{}]",           // LIDT — load IDT register from memory
        in(reg) idt_ptr,
        options(readonly, nostack, preserves_flags)
    );
}


// =============================================================================
// PART 5 — DEBUG & DIAGNOSTICS
// =============================================================================
//
// Read-only introspection of CPU state. All functions are safe because
// they only READ registers — no side effects, no memory writes, no
// privilege changes.
// =============================================================================

/// Read the current stack pointer (RSP).
///
/// Useful for debugging stack overflow, verifying stack alignment,
/// and validating context switch correctness.
///
/// # Why this is safe
/// MOV from RSP is a pure register read — no side effects.
///
/// # Note
/// The returned value is the RSP at the point of the asm instruction,
/// which includes the current function's stack frame.
#[inline(always)]
pub fn read_rsp() -> u64 {
    let rsp: u64;
    unsafe {
        asm!(
            "mov {}, rsp",
            out(reg) rsp,
            options(nomem, nostack)
        );
    }
    rsp
}

/// Read the RFLAGS register.
///
/// Returns the full 64-bit RFLAGS value. Key bits:
/// - Bit 0 (CF):  Carry flag
/// - Bit 6 (ZF):  Zero flag
/// - Bit 9 (IF):  Interrupt enable flag
/// - Bit 11 (OF): Overflow flag
///
/// # Why this is safe
/// PUSHFQ/POP only read CPU state — no side effects.
#[inline(always)]
pub fn read_rflags() -> u64 {
    let rflags: u64;
    unsafe {
        asm!(
            "pushfq",
            "pop {}",
            out(reg) rflags,
            options(nomem)
        );
    }
    rflags
}

/// Read the CR3 register (page table base address).
///
/// Returns the physical address of the current PML4 table.
/// Useful for verifying that context switches preserve the correct
/// address space, or for debugging page faults.
///
/// # Why this is safe
/// Reading CR3 is a privileged operation (Ring 0 only), but since
/// this code runs in the kernel, that's guaranteed. The read itself
/// has no side effects — it doesn't flush the TLB.
#[inline(always)]
pub fn read_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nomem, nostack, preserves_flags)
        );
    }
    cr3
}

/// Read the CR2 register (page fault linear address).
///
/// After a #PF exception, CR2 contains the virtual address that caused
/// the fault. Reading it at any other time returns the last fault address
/// (or 0 if no fault has occurred).
///
/// # Why this is safe
/// Reading CR2 is a pure register read — no side effects.
#[inline(always)]
pub fn read_cr2() -> u64 {
    let cr2: u64;
    unsafe {
        asm!(
            "mov {}, cr2",
            out(reg) cr2,
            options(nomem, nostack, preserves_flags)
        );
    }
    cr2
}

/// Invalidate the TLB entry for a specific virtual address.
///
/// # What it does
/// Removes the cached page table mapping for `addr` from the TLB.
/// The next access to `addr` will re-walk the page table.
///
/// # When to use
/// After modifying a page table entry for `addr` (remap, unmap, change flags).
///
/// # Safety
/// - `addr` should be a valid virtual address whose TLB entry needs flushing.
/// - Calling with an arbitrary address is harmless (just unnecessary TLB miss).
/// - On multi-core systems, this only flushes the LOCAL CPU's TLB.
#[inline(always)]
pub unsafe fn invlpg(addr: u64) {
    asm!(
        "invlpg [{}]",
        in(reg) addr,
        options(nostack, preserves_flags)
    );
}

/// Halt the CPU until the next interrupt.
///
/// # What it does
/// Enters a low-power state. The CPU will wake on ANY interrupt (even
/// if interrupts are disabled, an NMI will wake it).
///
/// # Why this is safe
/// HLT only stops the CPU temporarily — it resumes on the next interrupt.
/// No state is modified, no memory is written.
///
/// # When to use
/// In the kernel's idle loop to save power while waiting for interrupts.
#[inline(always)]
pub fn hlt() {
    unsafe {
        asm!(
            "hlt",
            options(nomem, nostack, preserves_flags)
        );
    }
}
