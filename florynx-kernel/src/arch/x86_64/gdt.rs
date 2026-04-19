// =============================================================================
// Florynx Kernel — Global Descriptor Table (GDT)
// =============================================================================
// Sets up the GDT with kernel code/data segments and a Task State Segment (TSS)
// for the double-fault handler stack.
// =============================================================================

use lazy_static::lazy_static;
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;

/// IST index for the double-fault handler stack.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Size of the kernel stack (128 KiB).
const STACK_SIZE: usize = 4096 * 32;
/// Ring0 privilege stack used when entering kernel from Ring3.
const PRIVILEGE_STACK_INDEX: usize = 0;

lazy_static! {
    /// Task State Segment with a dedicated double-fault stack.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // Required for CPL3->CPL0 transitions (e.g. int 0x80 from user mode).
        tss.privilege_stack_table[PRIVILEGE_STACK_INDEX] = {
            static mut PRIV_STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const PRIV_STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end
        };
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];
            let stack_start = VirtAddr::from_ptr(&raw const STACK);
            let stack_end = stack_start + STACK_SIZE;
            stack_end // Stack grows downward, so we point to the top
        };
        tss
    };
}

lazy_static! {
    /// Global Descriptor Table with kernel segments and TSS.
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let data_selector = gdt.add_entry(Descriptor::kernel_data_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        let user_data_selector = gdt.add_entry(Descriptor::user_data_segment());
        let user_code_selector = gdt.add_entry(Descriptor::user_code_segment());
        (
            gdt,
            Selectors {
                code_selector,
                data_selector,
                tss_selector,
                user_data_selector,
                user_code_selector,
            },
        )
    };
}

/// Holds the segment selectors for kernel code, data, and TSS.
#[derive(Debug, Clone, Copy)]
pub struct Selectors {
    pub code_selector: SegmentSelector,
    pub data_selector: SegmentSelector,
    pub tss_selector: SegmentSelector,
    pub user_data_selector: SegmentSelector,
    pub user_code_selector: SegmentSelector,
}

/// Load the GDT and set segment registers.
pub fn init() {
    use x86_64::instructions::segmentation::{CS, DS, ES, SS, Segment};
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        SS::set_reg(GDT.1.data_selector);
        load_tss(GDT.1.tss_selector);
    }

    crate::serial_println!("[gdt] loaded with kernel segments, TSS, and RSP0 stack");

    // Enable SSE and FPU now that segments are valid. Must happen before any
    // Rust-generated SSE instruction (common in optimized alloc/copy code).
    crate::arch::x86_64::cpu::enable_sse();
}
