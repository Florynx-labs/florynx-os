// =============================================================================
// Florynx Kernel — Device Filesystem (devfs)
// =============================================================================
// Virtual device files: /dev/null, /dev/zero, /dev/serial0
// =============================================================================

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    Null,
    Zero,
    Serial,
}

/// Read from a device
pub fn dev_read(device: DeviceType, buffer: &mut [u8]) -> usize {
    match device {
        DeviceType::Null => 0, // EOF
        DeviceType::Zero => {
            for b in buffer.iter_mut() {
                *b = 0;
            }
            buffer.len()
        }
        DeviceType::Serial => 0, // No input buffer yet
    }
}

/// Write to a device
pub fn dev_write(device: DeviceType, data: &[u8]) -> usize {
    match device {
        DeviceType::Null => data.len(), // Swallow everything
        DeviceType::Zero => 0,          // Can't write to /dev/zero
        DeviceType::Serial => {
            if let Ok(s) = core::str::from_utf8(data) {
                crate::serial_print!("{}", s);
            } else {
                for &b in data {
                    crate::serial_print!("{}", b as char);
                }
            }
            data.len()
        }
    }
}

/// Resolve a device path to a DeviceType
pub fn resolve(path: &str) -> Option<DeviceType> {
    match path {
        "/dev/null" | "dev/null" => Some(DeviceType::Null),
        "/dev/zero" | "dev/zero" => Some(DeviceType::Zero),
        "/dev/serial0" | "dev/serial0" => Some(DeviceType::Serial),
        _ => None,
    }
}

/// Initialize devfs
pub fn init() {
    crate::serial_println!("[devfs] initialized: /dev/null, /dev/zero, /dev/serial0");
}
