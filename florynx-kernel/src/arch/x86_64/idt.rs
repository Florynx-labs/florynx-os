// =============================================================================
// Florynx Kernel — Interrupt Descriptor Table (IDT)
// =============================================================================
// Configures the IDT with handlers for CPU exceptions and hardware IRQs.
// =============================================================================

use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};
use core::sync::atomic::{AtomicU64, Ordering};
use x86_64::PrivilegeLevel;

use crate::arch::x86_64::gdt;
use crate::interrupts::pic::InterruptIndex;
use x86_64::VirtAddr;

static PAGE_FAULT_TOTAL: AtomicU64 = AtomicU64::new(0);
static PAGE_FAULT_USER: AtomicU64 = AtomicU64::new(0);
static PAGE_FAULT_KERNEL: AtomicU64 = AtomicU64::new(0);
static SYSCALL_TOTAL: AtomicU64 = AtomicU64::new(0);
/// User RIP at the moment of the most recent int 0x80 call (single-core safe).
pub static SYSCALL_FRAME_RIP: AtomicU64 = AtomicU64::new(0);
/// User RSP at the moment of the most recent int 0x80 call.
pub static SYSCALL_FRAME_RSP: AtomicU64 = AtomicU64::new(0);
const SYSCALL_VECTOR: usize = 0x80;

lazy_static! {
    /// The global IDT instance.
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // CPU exceptions
        idt.divide_error.set_handler_fn(divide_error_handler);
        idt.debug.set_handler_fn(debug_handler);
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.invalid_opcode.set_handler_fn(invalid_opcode_handler);
        idt.stack_segment_fault.set_handler_fn(stack_segment_fault_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);
        idt.alignment_check.set_handler_fn(alignment_check_handler);
        idt.device_not_available.set_handler_fn(device_not_available_handler);

        // Hardware IRQs
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);
        unsafe {
            idt[SYSCALL_VECTOR]
                .set_handler_addr(VirtAddr::new(syscall_int80_stub as *const () as u64))
                .set_privilege_level(PrivilegeLevel::Ring3);
        }

        idt
    };
}

// ... handlers ...

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::handle_mouse_irq();

    unsafe {
        crate::interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Mouse.as_u8());
    }
}

/// Load the IDT into the CPU.
pub fn init() {
    IDT.load();
    crate::serial_println!("[idt] loaded with exception and IRQ handlers");
}

// ---------------------------------------------------------------------------
// CPU Exception Handlers
// ---------------------------------------------------------------------------

extern "x86-interrupt" fn device_not_available_handler(_stack_frame: InterruptStackFrame) {
    // Clear CR0.TS so the triggering FPU/SSE instruction can proceed.
    crate::arch::x86_64::cpu::clear_task_switched();
    // Save old FPU owner's state, restore current task's state.
    crate::process::scheduler::handle_fpu_fault();
}

extern "x86-interrupt" fn divide_error_handler(stack_frame: InterruptStackFrame) {
    use crate::core_kernel::exception::ExceptionContext;
    let ctx = ExceptionContext::new("DIVIDE ERROR", 0, &stack_frame);
    ctx.dump();
    panic!("DIVIDE BY ZERO");
}

extern "x86-interrupt" fn debug_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("[exception] DEBUG\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("[exception] BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn invalid_opcode_handler(stack_frame: InterruptStackFrame) {
    use crate::core_kernel::exception::ExceptionContext;
    let ctx = ExceptionContext::new("INVALID OPCODE", 6, &stack_frame);
    ctx.dump();
    panic!("INVALID OPCODE - Attempted to execute invalid instruction");
}

extern "x86-interrupt" fn stack_segment_fault_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    use crate::core_kernel::exception::ExceptionContext;
    let ctx = ExceptionContext::new("STACK SEGMENT FAULT", 12, &stack_frame);
    ctx.dump();
    crate::serial_println!("\n[exception] Error Code: 0x{:x}", error_code);
    panic!("STACK SEGMENT FAULT");
}

extern "x86-interrupt" fn alignment_check_handler(stack_frame: InterruptStackFrame, error_code: u64) {
    use crate::core_kernel::exception::ExceptionContext;
    let ctx = ExceptionContext::new("ALIGNMENT CHECK", 17, &stack_frame);
    ctx.dump();
    crate::serial_println!("\n[exception] Error Code: 0x{:x}", error_code);
    panic!("ALIGNMENT CHECK - Unaligned memory access");
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) -> ! {
    use crate::core_kernel::exception::ExceptionContext;
    
    let ctx = ExceptionContext::new("DOUBLE FAULT", 8, &stack_frame);
    ctx.dump();
    
    crate::serial_println!("\n[exception] DOUBLE FAULT - Error Code: 0x{:x}", error_code);
    crate::serial_println!("This usually indicates kernel stack overflow or nested exceptions.");
    crate::serial_println!("\nStack Frame:\n{:#?}", stack_frame);
    
    panic!("DOUBLE FAULT - UNRECOVERABLE");
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use crate::core_kernel::exception::{ExceptionContext, PageFaultInfo};
    use x86_64::registers::control::Cr2;
    use x86_64::structures::paging::PageTableFlags;

    let fault_addr = Cr2::read().as_u64();
    PAGE_FAULT_TOTAL.fetch_add(1, Ordering::Relaxed);
    let is_user_fault = error_code.contains(PageFaultErrorCode::USER_MODE)
        || ((stack_frame.code_segment & 0x3) == 0x3);
    let is_not_present = !error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION);

    if is_user_fault {
        PAGE_FAULT_USER.fetch_add(1, Ordering::Relaxed);

        // Demand paging: if the page is simply not present, allocate and map it.
        if is_not_present {
            let virt = x86_64::VirtAddr::new(fault_addr);
            let flags = PageTableFlags::PRESENT
                | PageTableFlags::WRITABLE
                | PageTableFlags::USER_ACCESSIBLE
                | PageTableFlags::NO_EXECUTE;
            if crate::memory::paging::map_page_now(virt, flags).is_ok() {
                return; // Page mapped — resume user instruction.
            }
        }

        crate::serial_println!(
            "[exception] user page fault: addr=0x{:x} err={:?} rip=0x{:x}",
            fault_addr,
            error_code,
            stack_frame.instruction_pointer.as_u64()
        );
        // Process Isolation: Force the offending user thread to terminate immediately
        // instead of returning and infinitely looping on the faulting instruction.
        crate::process::scheduler::exit_with_code(139); // SIGSEGV
    }

    PAGE_FAULT_KERNEL.fetch_add(1, Ordering::Relaxed);

    // Kernel guard page hit (stack overflow detection).
    if is_not_present {
        panic!(
            "KERNEL STACK OVERFLOW or NULL deref: guard page hit at 0x{:x} rip=0x{:x}",
            fault_addr,
            stack_frame.instruction_pointer.as_u64()
        );
    }

    let ctx = ExceptionContext::new("PAGE FAULT", 14, &stack_frame);
    ctx.dump();

    let pf_info = PageFaultInfo::new(error_code);
    pf_info.dump();

    crate::serial_println!(
        "\n[exception] PAGE FAULT - Stack Frame:\n{:#?}",
        stack_frame
    );
    panic!(
        "PAGE FAULT: addr=0x{:x} error={:?}",
        fault_addr,
        error_code
    );
}

#[derive(Debug, Clone, Copy)]
pub struct FaultTelemetry {
    pub page_fault_total: u64,
    pub page_fault_user: u64,
    pub page_fault_kernel: u64,
    pub syscall_total: u64,
}

pub fn fault_telemetry() -> FaultTelemetry {
    FaultTelemetry {
        page_fault_total: PAGE_FAULT_TOTAL.load(Ordering::Relaxed),
        page_fault_user: PAGE_FAULT_USER.load(Ordering::Relaxed),
        page_fault_kernel: PAGE_FAULT_KERNEL.load(Ordering::Relaxed),
        syscall_total: SYSCALL_TOTAL.load(Ordering::Relaxed),
    }
}

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    use crate::core_kernel::exception::ExceptionContext;
    
    let ctx = ExceptionContext::new("GENERAL PROTECTION FAULT", 13, &stack_frame);
    ctx.dump();
    
    crate::serial_println!("\n[exception] GPF - Error Code: 0x{:x}", error_code);
    if error_code != 0 {
        let segment = (error_code & 0xFFF8) >> 3;
        let table = (error_code & 0x6) >> 1;
        let external = error_code & 0x1;
        crate::serial_println!("  Segment: 0x{:x}, Table: {}, External: {}", 
            segment, table, external);
    }
    crate::serial_println!("\nStack Frame:\n{:#?}", stack_frame);
    
    panic!("GENERAL PROTECTION FAULT");
}

// ---------------------------------------------------------------------------
// Hardware IRQ Handlers
// ---------------------------------------------------------------------------

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::handle_timer_irq();

    // Notify the scheduler
    crate::process::scheduler::timer_tick();

    // Pump driver events → GUI event bus (try_lock, bounded, ISR-safe).
    crate::drivers::try_process_events();

    // Frame-limited GUI redraw (~60 FPS at 200 Hz PIT = every 3 ticks).
    static FRAME_TICK: AtomicU64 = AtomicU64::new(0);
    if FRAME_TICK.fetch_add(1, Ordering::Relaxed) % 3 == 0 {
        crate::gui::desktop::redraw_if_needed();
    }

    // Send End-Of-Interrupt to the PIC
    unsafe {
        crate::interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::handle_keyboard_irq();

    unsafe {
        crate::interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

// ---------------------------------------------------------------------------
// Syscall int 0x80 — raw assembly ISR stub
// ---------------------------------------------------------------------------
// The x86-interrupt calling convention clobbers GP registers in the prologue,
// so an inline-asm read of rax/rdi/rsi/rdx inside such a handler returns
// compiler-generated values, not the user's syscall arguments.
//
// This stub:
//   1. Saves ALL GP registers (scratch + callee-saved).
//   2. Passes the user's rax, rdi, rsi, rdx to the Rust dispatch function.
//   3. Writes the i64 return value into the saved-rax slot so the user sees
//      it after iretq.
//   4. Restores everything and returns with iretq.
// ---------------------------------------------------------------------------

/// Rust-side dispatch called from the asm stub.
/// NOT pub — only the asm stub references this symbol.
/// r8 = user_rip, r9 = user_rsp extracted from the iretq frame.
extern "C" fn syscall_int80_inner(
    nr: u64, arg1: u64, arg2: u64, arg3: u64,
    user_rip: u64, user_rsp: u64,
) -> i64 {
    SYSCALL_TOTAL.fetch_add(1, Ordering::Relaxed);
    SYSCALL_FRAME_RIP.store(user_rip, Ordering::Relaxed);
    SYSCALL_FRAME_RSP.store(user_rsp, Ordering::Relaxed);
    crate::syscall::dispatch(nr, arg1, arg2, arg3)
}

extern "C" { fn syscall_int80_stub(); }

core::arch::global_asm!(
    ".global syscall_int80_stub",
    "syscall_int80_stub:",
    // Save all GP registers (order matters for restore)
    "push rax",   // [rsp+0x70] — will be overwritten with return value
    "push rcx",
    "push rdx",
    "push rbx",
    "push rbp",
    "push rsi",
    "push rdi",
    "push r8",
    "push r9",
    "push r10",
    "push r11",
    "push r12",
    "push r13",
    "push r14",
    "push r15",
    // Load user RIP and RSP from the iretq frame sitting above the 15 saved GPs.
    // After 15 × push (120 bytes), the iretq frame is at [rsp+120..rsp+160]:
    //   [rsp+120] = user RIP, [rsp+128] = CS, [rsp+136] = RFLAGS,
    //   [rsp+144] = user RSP, [rsp+152] = SS
    // SysV calling convention: arg5 in r8, arg6 in r9.
    "mov r8, [rsp + 120]",  // user RIP  → SysV 5th arg
    "mov r9, [rsp + 144]",  // user RSP  → SysV 6th arg
    // SysV arg registers: rdi=nr, rsi=arg1, rdx=arg2, rcx=arg3
    // User passed: rax=nr, rdi=arg1, rsi=arg2, rdx=arg3
    "mov rcx, rdx",   // arg3 (user rdx -> SysV rcx)
    "mov rdx, rsi",   // arg2 (user rsi -> SysV rdx)
    "mov rsi, rdi",   // arg1 (user rdi -> SysV rsi)
    "mov rdi, rax",   // nr   (user rax -> SysV rdi)
    "call {handler}",
    // Return value is in rax. Store it into the saved-rax slot on the stack
    // so the pop sequence below restores it into the user's rax.
    "mov [rsp + 14*8], rax",
    // Restore GP registers
    "pop r15",
    "pop r14",
    "pop r13",
    "pop r12",
    "pop r11",
    "pop r10",
    "pop r9",
    "pop r8",
    "pop rdi",
    "pop rsi",
    "pop rbp",
    "pop rbx",
    "pop rdx",
    "pop rcx",
    "pop rax",  // now contains the return value
    "iretq",
    handler = sym syscall_int80_inner,
);
