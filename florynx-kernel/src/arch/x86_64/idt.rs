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
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        }
        idt.page_fault.set_handler_fn(page_fault_handler);
        idt.general_protection_fault.set_handler_fn(general_protection_fault_handler);

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

extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    crate::serial_println!("[exception] BREAKPOINT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    crate::serial_println!("[exception] DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("DOUBLE FAULT\n{:#?}", stack_frame);
}

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;

    crate::serial_println!(
        "[exception] PAGE FAULT\nAccessed address: {:?}\nError code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
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
    crate::serial_println!(
        "[exception] GENERAL PROTECTION FAULT\nError code: {}\n{:#?}",
        error_code,
        stack_frame
    );
    panic!("GENERAL PROTECTION FAULT: error_code={}", error_code);
}

// ---------------------------------------------------------------------------
// Hardware IRQ Handlers
// ---------------------------------------------------------------------------

extern "x86-interrupt" fn timer_interrupt_handler(_stack_frame: InterruptStackFrame) {
    // Increment the PIT tick counter
    crate::drivers::timer::pit::tick();

    // Notify the scheduler (if running)
    crate::process::scheduler::timer_tick();

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
