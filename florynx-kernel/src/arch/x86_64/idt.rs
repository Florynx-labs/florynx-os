// =============================================================================
// Florynx Kernel — Interrupt Descriptor Table (IDT)
// =============================================================================
// Configures the IDT with handlers for CPU exceptions and hardware IRQs.
// =============================================================================

use lazy_static::lazy_static;
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame, PageFaultErrorCode};

use crate::arch::x86_64::gdt;
use crate::interrupts::pic::InterruptIndex;

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

        // Hardware IRQs
        idt[InterruptIndex::Timer.as_usize()].set_handler_fn(timer_interrupt_handler);
        idt[InterruptIndex::Keyboard.as_usize()].set_handler_fn(keyboard_interrupt_handler);
        idt[InterruptIndex::Mouse.as_usize()].set_handler_fn(mouse_interrupt_handler);

        idt
    };
}

// ... handlers ...

extern "x86-interrupt" fn mouse_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::input::mouse::handle_interrupt();

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

    let ctx = ExceptionContext::new("PAGE FAULT", 14, &stack_frame);
    ctx.dump();
    
    let pf_info = PageFaultInfo::new(error_code);
    pf_info.dump();

    use x86_64::registers::control::Cr2;
    
    crate::serial_println!(
        "\n[exception] PAGE FAULT - Stack Frame:\n{:#?}",
        stack_frame
    );
    panic!(
        "PAGE FAULT: addr={:?} error={:?}",
        Cr2::read(),
        error_code
    );
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
    // Increment the PIT tick counter
    crate::drivers::timer::pit::tick();

    // Notify the old scheduler (if running)
    crate::process::scheduler::timer_tick();
    
    // Notify the new scheduler (v2)
    crate::process::scheduler_v2::timer_tick();

    // Send End-Of-Interrupt to the PIC
    unsafe {
        crate::interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}

extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    crate::drivers::input::keyboard::handle_keyboard_interrupt();

    unsafe {
        crate::interrupts::pic::PICS
            .lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}
