// =============================================================================
// Florynx Shared — Syscall ABI
// =============================================================================
// Syscall numbers and argument conventions shared between kernel and userland.
// =============================================================================

// --- Syscall numbers ---
pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_ABI_INFO: u64 = 0x00F0;
pub const SYS_DEBUG_TELEMETRY: u64 = 0x00F1;
pub const SYS_OPEN: u64 = 2;
pub const SYS_CLOSE: u64 = 3;
pub const SYS_STAT: u64 = 4;
pub const SYS_SEEK: u64 = 8;
pub const SYS_YIELD: u64 = 24;
pub const SYS_SLEEP: u64 = 35;
pub const SYS_GETPID: u64 = 39;
pub const SYS_EXIT: u64 = 60;
pub const SYS_WAIT: u64 = 61;
pub const SYS_KILL: u64 = 62;
pub const SYS_MKDIR: u64 = 83;

// --- GUI syscalls (Florynx extension, 0x1000+) ---
pub const SYS_GUI_CREATE_WINDOW: u64 = 0x1000;
pub const SYS_GUI_DESTROY_WINDOW: u64 = 0x1001;
pub const SYS_GUI_DRAW_RECT: u64 = 0x1002;
pub const SYS_GUI_DRAW_TEXT: u64 = 0x1003;
pub const SYS_GUI_POLL_EVENT: u64 = 0x1004;
pub const SYS_GUI_SET_WALLPAPER: u64 = 0x1005;
pub const SYS_GUI_INVALIDATE: u64 = 0x1006;
pub const SYS_GUI_FOCUS_WINDOW: u64 = 0x1007;
pub const SYS_GUI_BLIT_BUFFER: u64 = 0x1008;

// --- IPC syscalls (0x2000+) ---
pub const SYS_IPC_SEND: u64 = 0x2000;
pub const SYS_IPC_RECV: u64 = 0x2001;
pub const SYS_IPC_SUBSCRIBE: u64 = 0x2002;

// --- Error codes ---
pub const E_OK: i64 = 0;
pub const E_PERM: i64 = -1;
pub const E_NOENT: i64 = -2;
pub const E_IO: i64 = -5;
pub const E_CHILD: i64 = -10;
pub const E_AGAIN: i64 = -11;
pub const E_NOMEM: i64 = -12;
pub const E_INVAL: i64 = -22;
pub const E_NOSYS: i64 = -38;

// --- ABI struct headers ---
pub const ABI_V1: u16 = 1;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AbiHeader {
    pub size: u16,
    pub version: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct AbiInfoV1 {
    pub hdr: AbiHeader,
    pub abi_major: u16,
    pub abi_minor: u16,
    pub user_stat_size: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct UserStatV1 {
    pub hdr: AbiHeader,
    pub _pad: u32, // Explicitly zeroed padding to prevent stack leak
    pub inode: u64,
    pub size: u64,
    pub file_type: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct KernelTelemetryV1 {
    pub hdr: AbiHeader,
    pub _pad: u32, // Explicitly zeroed padding to prevent stack leak
    pub page_fault_total: u64,
    pub page_fault_user: u64,
    pub page_fault_kernel: u64,
    pub panic_count: u64,
}
