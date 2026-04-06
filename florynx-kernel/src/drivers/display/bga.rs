// =============================================================================
// Florynx Kernel — BGA (Bochs Graphic Adapter) Driver
// =============================================================================

use x86_64::instructions::port::Port;
use crate::drivers::display::framebuffer::{self, PixelFormat};

const VBE_DISPI_IOPORT_INDEX: u16 = 0x01CE;
const VBE_DISPI_IOPORT_DATA: u16 = 0x01CF;

const VBE_DISPI_INDEX_ID: u16 = 0;
const VBE_DISPI_INDEX_XRES: u16 = 1;
const VBE_DISPI_INDEX_YRES: u16 = 2;
const VBE_DISPI_INDEX_BPP: u16 = 3;
const VBE_DISPI_INDEX_ENABLE: u16 = 4;

const VBE_DISPI_DISABLED: u16 = 0x00;
const VBE_DISPI_ENABLED: u16 = 0x01;
const VBE_DISPI_LFB_ENABLED: u16 = 0x40;

/// PCI Configuration Ports
const PCI_CONFIG_ADDRESS: u16 = 0x0CF8;
const PCI_CONFIG_DATA: u16 = 0x0CFC;

fn pci_read_u32(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x80000000;
    let mut addr_port = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::new(PCI_CONFIG_DATA);
    unsafe {
        addr_port.write(address);
        data_port.read()
    }
}

fn bga_write(index: u16, data: u16) {
    let mut index_port = Port::new(VBE_DISPI_IOPORT_INDEX);
    let mut data_port = Port::new(VBE_DISPI_IOPORT_DATA);
    unsafe {
        index_port.write(index);
        data_port.write(data);
    }
}

fn bga_read(index: u16) -> u16 {
    let mut index_port = Port::new(VBE_DISPI_IOPORT_INDEX);
    let mut data_port = Port::new(VBE_DISPI_IOPORT_DATA);
    unsafe {
        index_port.write(index);
        data_port.read()
    }
}

/// Check if BGA is available and find its framebuffer address via PCI.
pub fn find_bga_address() -> Option<u64> {
    // Basic PCI scan of bus 0 for Vid 0x1234, Did 0x1111 (BGA)
    for slot in 0..32 {
        let vendor_id = (pci_read_u32(0, slot, 0, 0) & 0xFFFF) as u16;
        let device_id = (pci_read_u32(0, slot, 0, 0) >> 16) as u16;
        
        if vendor_id == 0x1234 && device_id == 0x1111 {
            // BAR 0 contains the LFB address
            let bar0 = pci_read_u32(0, slot, 0, 0x10);
            return Some((bar0 & 0xFFFFFFF0) as u64);
        }
    }
    None
}

/// Initialize BGA and set up the global framebuffer.
pub fn init(physical_memory_offset: u64) {
    if bga_read(VBE_DISPI_INDEX_ID) < 0xB0C0 {
        crate::serial_println!("[bga] device not found or incompatible");
        return;
    }

    let width = 1024;
    let height = 768;
    let bpp = 32;

    bga_write(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_DISABLED);
    bga_write(VBE_DISPI_INDEX_XRES, width as u16);
    bga_write(VBE_DISPI_INDEX_YRES, height as u16);
    bga_write(VBE_DISPI_INDEX_BPP, bpp as u16);
    bga_write(VBE_DISPI_INDEX_ENABLE, VBE_DISPI_ENABLED | VBE_DISPI_LFB_ENABLED);
    
    // Find the actual framebuffer address via PCI — do NOT use a fallback
    let lfb_phys = match find_bga_address() {
        Some(addr) => addr,
        None => {
            crate::serial_println!("[bga] WARNING: could not find BGA PCI device, skipping framebuffer");
            return;
        }
    };
    
    // APPLY PHYSICAL MEMORY OFFSET
    let lfb_virt = physical_memory_offset + lfb_phys;
    let lfb_ptr = lfb_virt as *mut u8;

    unsafe {
        framebuffer::init(lfb_ptr, width, height, width, PixelFormat::BGR);
    }

    crate::serial_println!("[bga] set mode {}x{} at {:#x} (virt {:#x})", width, height, lfb_phys, lfb_virt);
}
