// =============================================================================
// Florynx Kernel — Syscall User Memory Helpers
// =============================================================================
// Centralized pointer/slice validation for syscall handlers.
// Current implementation is conservative and non-breaking: it validates
// null/overflow/canonical form and bounds length.
// =============================================================================

use alloc::vec::Vec;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{PageTable, PageTableFlags};
use x86_64::VirtAddr;

const EFAULT: i64 = -14;
const EINVAL: i64 = -22;
const MAX_SYSCALL_COPY: u64 = 64 * 1024;
const USER_MAX_VADDR: u64 = 0x0000_7FFF_FFFF_FFFF;

#[derive(Clone, Copy)]
enum Access {
    Read,
    Write,
}

#[inline]
pub fn validate_user_ptr(ptr: u64, len: u64) -> bool {
    if ptr == 0 { return false; }
    let end = match ptr.checked_add(len) {
        Some(e) => e,
        None => return false,
    };
    if !VirtAddr::try_new(ptr).is_ok() || (len > 0 && !VirtAddr::try_new(end - 1).is_ok()) {
        return false;
    }
    if ptr > USER_MAX_VADDR || (len > 0 && (end - 1) > USER_MAX_VADDR) {
        return false;
    }
    true
}

#[inline]
fn validate_user_range(ptr: u64, len: u64, access: Access) -> Result<(), i64> {
    if !validate_user_ptr(ptr, len) {
        return Err(EFAULT);
    }
    if len == 0 || len > MAX_SYSCALL_COPY {
        return Err(EINVAL);
    }

    validate_page_permissions(ptr, len, access)?;

    Ok(())
}

pub fn copy_from_user(ptr: u64, len: u64) -> Result<Vec<u8>, i64> {
    validate_user_range(ptr, len, Access::Read)?;
    let mut out = Vec::new();
    out.resize(len as usize, 0);
    unsafe {
        core::ptr::copy_nonoverlapping(ptr as *const u8, out.as_mut_ptr(), len as usize);
    }
    Ok(out)
}

pub fn copy_to_user(ptr: u64, src: &[u8]) -> Result<(), i64> {
    let len = src.len() as u64;
    validate_user_range(ptr, len, Access::Write)?;
    unsafe {
        core::ptr::copy_nonoverlapping(src.as_ptr(), ptr as *mut u8, src.len());
    }
    Ok(())
}

fn validate_page_permissions(ptr: u64, len: u64, access: Access) -> Result<(), i64> {
    let phys_offset = crate::memory::paging::physical_memory_offset().ok_or(EFAULT)?;
    let (l4_frame, _) = Cr3::read();
    let l4 = unsafe { page_table_from_phys(l4_frame.start_address().as_u64(), phys_offset.as_u64()) };

    let start = ptr;
    let end = ptr + len - 1;
    let mut addr = start & !0xFFF;
    while addr <= end {
        validate_single_page(addr, l4, phys_offset.as_u64(), access)?;
        if addr > u64::MAX - 0x1000 {
            break;
        }
        addr += 0x1000;
    }
    Ok(())
}

fn validate_single_page(
    vaddr: u64,
    l4: &PageTable,
    phys_offset: u64,
    access: Access,
) -> Result<(), i64> {
    let i4 = ((vaddr >> 39) & 0x1FF) as usize;
    let e4 = &l4[i4];
    if !e4.flags().contains(PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE) {
        return Err(EFAULT);
    }
    let l3 = unsafe { page_table_from_phys(e4.addr().as_u64(), phys_offset) };

    let i3 = ((vaddr >> 30) & 0x1FF) as usize;
    let e3 = &l3[i3];
    let f3 = e3.flags();
    if !f3.contains(PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE) {
        return Err(EFAULT);
    }
    if f3.contains(PageTableFlags::HUGE_PAGE) {
        if matches!(access, Access::Write) && !f3.contains(PageTableFlags::WRITABLE) {
            return Err(EFAULT);
        }
        return Ok(());
    }
    let l2 = unsafe { page_table_from_phys(e3.addr().as_u64(), phys_offset) };

    let i2 = ((vaddr >> 21) & 0x1FF) as usize;
    let e2 = &l2[i2];
    let f2 = e2.flags();
    if !f2.contains(PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE) {
        return Err(EFAULT);
    }
    if f2.contains(PageTableFlags::HUGE_PAGE) {
        if matches!(access, Access::Write) && !f2.contains(PageTableFlags::WRITABLE) {
            return Err(EFAULT);
        }
        return Ok(());
    }
    let l1 = unsafe { page_table_from_phys(e2.addr().as_u64(), phys_offset) };

    let i1 = ((vaddr >> 12) & 0x1FF) as usize;
    let e1 = &l1[i1];
    let f1 = e1.flags();
    if !f1.contains(PageTableFlags::PRESENT | PageTableFlags::USER_ACCESSIBLE) {
        return Err(EFAULT);
    }
    if matches!(access, Access::Write) && !f1.contains(PageTableFlags::WRITABLE) {
        return Err(EFAULT);
    }

    Ok(())
}

/// Read a null-terminated C string from user space (up to `max_len` bytes).
/// Returns `Err(EFAULT)` if the pointer is invalid, `Err(EINVAL)` if no NUL
/// found within `max_len`.
pub fn read_cstr_from_user(ptr: u64, max_len: usize) -> Result<alloc::string::String, i64> {
    if !validate_user_ptr(ptr, 1) { return Err(EFAULT); }
    
    let mut result = alloc::string::String::new();
    for i in 0..max_len {
        let addr = ptr.checked_add(i as u64).ok_or(EFAULT)?;
        if !validate_user_ptr(addr, 1) { return Err(EFAULT); }
        
        // Check page permission for each byte (or optimized per page)
        // For simplicity and safety in strings, we validate as we go.
        validate_page_permissions(addr, 1, Access::Read)?;

        let byte = unsafe { *(addr as *const u8) };
        if byte == 0 { return Ok(result); }
        result.push(byte as char);
    }
    Err(EINVAL) // no NUL found within max_len
}

unsafe fn page_table_from_phys(phys_addr: u64, phys_offset: u64) -> &'static PageTable {
    let virt = VirtAddr::new(phys_offset + phys_addr);
    &*(virt.as_u64() as *const PageTable)
}

