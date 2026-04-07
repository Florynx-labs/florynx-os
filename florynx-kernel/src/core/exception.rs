// =============================================================================
// Florynx Kernel — Enhanced Exception Handling
// =============================================================================
// Production-level exception handlers with detailed error reporting,
// CPU state dumps, and stack traces for debugging.
// =============================================================================

use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
use x86_64::registers::control::{Cr2, Cr3};
use x86_64::VirtAddr;

/// CPU register state at the time of exception
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct CpuState {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub rsp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rip: u64,
    pub rflags: u64,
    pub cr2: u64,
    pub cr3: u64,
}

impl CpuState {
    /// Capture current CPU state (partial - from stack frame)
    pub fn from_stack_frame(frame: &InterruptStackFrame) -> Self {
        CpuState {
            rax: 0, // Not available in InterruptStackFrame
            rbx: 0,
            rcx: 0,
            rdx: 0,
            rsi: 0,
            rdi: 0,
            rbp: 0,
            rsp: frame.stack_pointer.as_u64(),
            r8: 0,
            r9: 0,
            r10: 0,
            r11: 0,
            r12: 0,
            r13: 0,
            r14: 0,
            r15: 0,
            rip: frame.instruction_pointer.as_u64(),
            rflags: frame.cpu_flags,
            cr2: Cr2::read_raw(),
            cr3: Cr3::read().0.start_address().as_u64(),
        }
    }

    /// Print CPU state to serial
    pub fn dump(&self) {
        crate::serial_println!("╔════════════════════════════════════════════════════════════════╗");
        crate::serial_println!("║                    CPU STATE DUMP                              ║");
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");
        crate::serial_println!("║ RAX: 0x{:016x}  RBX: 0x{:016x}              ║", self.rax, self.rbx);
        crate::serial_println!("║ RCX: 0x{:016x}  RDX: 0x{:016x}              ║", self.rcx, self.rdx);
        crate::serial_println!("║ RSI: 0x{:016x}  RDI: 0x{:016x}              ║", self.rsi, self.rdi);
        crate::serial_println!("║ RBP: 0x{:016x}  RSP: 0x{:016x}              ║", self.rbp, self.rsp);
        crate::serial_println!("║ R8:  0x{:016x}  R9:  0x{:016x}              ║", self.r8, self.r9);
        crate::serial_println!("║ R10: 0x{:016x}  R11: 0x{:016x}              ║", self.r10, self.r11);
        crate::serial_println!("║ R12: 0x{:016x}  R13: 0x{:016x}              ║", self.r12, self.r13);
        crate::serial_println!("║ R14: 0x{:016x}  R15: 0x{:016x}              ║", self.r14, self.r15);
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");
        crate::serial_println!("║ RIP: 0x{:016x}  RFLAGS: 0x{:016x}          ║", self.rip, self.rflags);
        crate::serial_println!("║ CR2: 0x{:016x}  CR3:    0x{:016x}          ║", self.cr2, self.cr3);
        crate::serial_println!("╚════════════════════════════════════════════════════════════════╝");
    }
}

/// Page fault error information
#[derive(Debug)]
pub struct PageFaultInfo {
    pub address: VirtAddr,
    pub error_code: PageFaultErrorCode,
    pub present: bool,
    pub write: bool,
    pub user: bool,
    pub reserved_write: bool,
    pub instruction_fetch: bool,
}

impl PageFaultInfo {
    pub fn new(error_code: PageFaultErrorCode) -> Self {
        let address = Cr2::read();
        PageFaultInfo {
            address,
            error_code,
            present: !error_code.contains(PageFaultErrorCode::PROTECTION_VIOLATION),
            write: error_code.contains(PageFaultErrorCode::CAUSED_BY_WRITE),
            user: error_code.contains(PageFaultErrorCode::USER_MODE),
            reserved_write: error_code.contains(PageFaultErrorCode::MALFORMED_TABLE),
            instruction_fetch: error_code.contains(PageFaultErrorCode::INSTRUCTION_FETCH),
        }
    }

    pub fn dump(&self) {
        crate::serial_println!("╔════════════════════════════════════════════════════════════════╗");
        crate::serial_println!("║                   PAGE FAULT ANALYSIS                          ║");
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");
        crate::serial_println!("║ Faulting Address: 0x{:016x}                          ║", self.address.as_u64());
        crate::serial_println!("║ Error Code:       0x{:04x}                                      ║", self.error_code.bits());
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");
        crate::serial_println!("║ Reason:                                                        ║");
        
        if !self.present {
            crate::serial_println!("║   ✗ Page not present (unmapped memory)                        ║");
        } else {
            crate::serial_println!("║   ✓ Page present (protection violation)                       ║");
        }
        
        if self.write {
            crate::serial_println!("║   ✗ Write access violation                                     ║");
        } else {
            crate::serial_println!("║   ○ Read access                                                ║");
        }
        
        if self.user {
            crate::serial_println!("║   ✗ User mode access (privilege violation)                     ║");
        } else {
            crate::serial_println!("║   ○ Kernel mode access                                         ║");
        }
        
        if self.reserved_write {
            crate::serial_println!("║   ✗ Reserved bits set in page table                            ║");
        }
        
        if self.instruction_fetch {
            crate::serial_println!("║   ✗ Instruction fetch (execute violation)                      ║");
        }
        
        crate::serial_println!("╚════════════════════════════════════════════════════════════════╝");
    }
}

/// Stack trace walker
pub struct StackTrace {
    rbp: u64,
    depth: usize,
}

impl StackTrace {
    pub fn new(rbp: u64) -> Self {
        StackTrace { rbp, depth: 0 }
    }

    /// Walk the stack and print return addresses
    pub fn walk(&mut self, max_depth: usize) {
        crate::serial_println!("╔════════════════════════════════════════════════════════════════╗");
        crate::serial_println!("║                      STACK TRACE                               ║");
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");

        while self.depth < max_depth && self.rbp != 0 {
            // Read return address from stack (rbp + 8)
            let return_addr = unsafe {
                let ptr = (self.rbp + 8) as *const u64;
                if self.is_valid_kernel_address(ptr as u64) {
                    *ptr
                } else {
                    break;
                }
            };

            crate::serial_println!("║ #{:2}: RIP = 0x{:016x}                                  ║", 
                self.depth, return_addr);

            // Move to previous frame
            let prev_rbp = unsafe {
                let ptr = self.rbp as *const u64;
                if self.is_valid_kernel_address(ptr as u64) {
                    *ptr
                } else {
                    break;
                }
            };

            if prev_rbp <= self.rbp {
                break; // Prevent infinite loop
            }

            self.rbp = prev_rbp;
            self.depth += 1;
        }

        if self.depth == 0 {
            crate::serial_println!("║ (No stack frames available)                                   ║");
        }

        crate::serial_println!("╚════════════════════════════════════════════════════════════════╝");
    }

    /// Check if address is in valid kernel range
    fn is_valid_kernel_address(&self, addr: u64) -> bool {
        // Kernel addresses are typically in higher half
        addr >= 0xFFFF_8000_0000_0000 && addr < 0xFFFF_FFFF_FFFF_FFFF
    }
}

/// Exception context for detailed error reporting
pub struct ExceptionContext {
    pub name: &'static str,
    pub vector: u8,
    pub cpu_state: CpuState,
}

impl ExceptionContext {
    pub fn new(name: &'static str, vector: u8, frame: &InterruptStackFrame) -> Self {
        ExceptionContext {
            name,
            vector,
            cpu_state: CpuState::from_stack_frame(frame),
        }
    }

    pub fn dump(&self) {
        crate::serial_println!("\n");
        crate::serial_println!("╔════════════════════════════════════════════════════════════════╗");
        crate::serial_println!("║                  KERNEL EXCEPTION                              ║");
        crate::serial_println!("╠════════════════════════════════════════════════════════════════╣");
        crate::serial_println!("║ Exception: {}                                              ", self.name);
        crate::serial_println!("║ Vector:    0x{:02x}                                                 ║", self.vector);
        crate::serial_println!("╚════════════════════════════════════════════════════════════════╝");
        crate::serial_println!("\n");
        
        self.cpu_state.dump();
        
        crate::serial_println!("\n");
        let mut stack_trace = StackTrace::new(self.cpu_state.rbp);
        stack_trace.walk(10);
    }
}
