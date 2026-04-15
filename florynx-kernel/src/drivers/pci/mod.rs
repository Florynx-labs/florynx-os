// =============================================================================
// Florynx Kernel — PCI Configuration Space Scanner
// =============================================================================
// Uses the legacy CAM (Configuration Access Mechanism) via I/O ports
// 0xCF8 (address) and 0xCFC (data) to enumerate PCI devices.
//
// Supports bus 0..=255, device 0..=31, function 0..=7.
// Only the minimal header fields needed to locate virtio-blk are decoded.
// =============================================================================

use x86_64::instructions::port::Port;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// Vendor / device IDs we care about
// ---------------------------------------------------------------------------

/// Virtio vendor ID (Red Hat / QEMU).
pub const VIRTIO_VENDOR: u16 = 0x1AF4;
/// Virtio-blk legacy transitional device ID.
pub const VIRTIO_BLK_LEGACY: u16 = 0x1001;
/// Virtio-blk modern device ID.
pub const VIRTIO_BLK_MODERN: u16 = 0x1042;

// ---------------------------------------------------------------------------
// Low-level CAM accessors
// ---------------------------------------------------------------------------

/// Build the 32-bit PCI CONFIG_ADDRESS value.
#[inline]
fn pci_addr(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    (1u32 << 31)
        | ((bus as u32) << 16)
        | ((dev as u32) << 11)
        | ((func as u32) << 8)
        | ((offset as u32) & 0xFC)
}

/// Read a 32-bit DWORD from PCI config space.
pub fn read_u32(bus: u8, dev: u8, func: u8, offset: u8) -> u32 {
    let mut addr_port: Port<u32> = Port::new(0xCF8);
    let mut data_port: Port<u32> = Port::new(0xCFC);
    unsafe {
        addr_port.write(pci_addr(bus, dev, func, offset));
        data_port.read()
    }
}

/// Read a 16-bit WORD from PCI config space.
#[inline]
pub fn read_u16(bus: u8, dev: u8, func: u8, offset: u8) -> u16 {
    let dword = read_u32(bus, dev, func, offset & !3);
    let shift = (offset & 2) * 8;
    (dword >> shift) as u16
}

/// Read an 8-bit BYTE from PCI config space.
#[inline]
pub fn read_u8(bus: u8, dev: u8, func: u8, offset: u8) -> u8 {
    let dword = read_u32(bus, dev, func, offset & !3);
    let shift = (offset & 3) * 8;
    (dword >> shift) as u8
}

// ---------------------------------------------------------------------------
// Device descriptor
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class: u8,
    pub subclass: u8,
    /// BAR0 raw value (caller must strip flags to get address).
    pub bar0: u32,
    pub irq_line: u8,
}

impl PciDevice {
    /// Returns the I/O-space base address from BAR0 (bit0=1 means I/O space).
    pub fn bar0_io_base(&self) -> Option<u16> {
        if self.bar0 & 1 == 1 {
            Some((self.bar0 & !0x3) as u16)
        } else {
            None
        }
    }

    /// Returns the MMIO base address from BAR0 (bit0=0 means memory space).
    pub fn bar0_mmio_base(&self) -> Option<u64> {
        if self.bar0 & 1 == 0 {
            // 64-bit BAR type = bits[2:1] == 0b10
            Some((self.bar0 & !0xF) as u64)
        } else {
            None
        }
    }
}

// ---------------------------------------------------------------------------
// Enumeration
// ---------------------------------------------------------------------------

/// Scan all PCI buses / devices / functions and return every present device.
///
/// This performs a full 256×32×8 scan; on real hardware it can take a few ms.
/// Only call it once at boot and cache the result.
pub fn enumerate() -> Vec<PciDevice> {
    let mut found = Vec::new();

    for bus in 0u8..=255 {
        for dev in 0u8..32 {
            // Check function 0 first; if vendor == 0xFFFF the slot is empty.
            let id0 = read_u32(bus, dev, 0, 0x00);
            let vendor0 = (id0 & 0xFFFF) as u16;
            if vendor0 == 0xFFFF {
                continue;
            }

            let hdr_type = read_u8(bus, dev, 0, 0x0E);
            let multi_function = hdr_type & 0x80 != 0;
            let func_count: u8 = if multi_function { 8 } else { 1 };

            for func in 0..func_count {
                let id = read_u32(bus, dev, func, 0x00);
                let vendor_id = (id & 0xFFFF) as u16;
                if vendor_id == 0xFFFF {
                    continue;
                }
                let device_id = (id >> 16) as u16;
                let class_dword = read_u32(bus, dev, func, 0x08);
                let class   = ((class_dword >> 24) & 0xFF) as u8;
                let subclass = ((class_dword >> 16) & 0xFF) as u8;
                let bar0    = read_u32(bus, dev, func, 0x10);
                let irq_line = read_u8(bus, dev, func, 0x3C);

                found.push(PciDevice {
                    bus, dev, func,
                    vendor_id, device_id,
                    class, subclass,
                    bar0, irq_line,
                });
            }
        }
    }

    found
}

/// Find the first virtio-blk device (legacy or modern) in the PCI enumeration.
pub fn find_virtio_blk() -> Option<PciDevice> {
    enumerate().into_iter().find(|d| {
        d.vendor_id == VIRTIO_VENDOR
            && (d.device_id == VIRTIO_BLK_LEGACY || d.device_id == VIRTIO_BLK_MODERN)
    })
}
