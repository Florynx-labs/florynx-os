// =============================================================================
// Florynx Kernel — Process Structure
// =============================================================================
// Represents a process — a container for one or more tasks with its own
// address space and resource accounting.
// =============================================================================

use alloc::string::String;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::structures::paging::PhysFrame;
use x86_64::PhysAddr;
use crate::security::isolation::IsolationDomain;
use crate::security::capability::CapabilityTable;

use super::task::TaskId;

/// Unique process identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessId(pub u64);

static NEXT_PID: AtomicU64 = AtomicU64::new(1);
static CLEANUP_EVENTS: AtomicU64 = AtomicU64::new(0);
static CLEANUP_FDS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CLEANUP_LINKS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CLEANUP_REGIONS_TOTAL: AtomicU64 = AtomicU64::new(0);
static CLEANUP_PT_TOTAL: AtomicU64 = AtomicU64::new(0);

impl ProcessId {
    pub fn new() -> Self {
        ProcessId(NEXT_PID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Process state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    Created,
    Running,
    Sleeping,
    Zombie,
}

/// A process — owns an address space and a set of tasks.
pub struct Process {
    pub pid: ProcessId,
    pub name: String,
    pub state: ProcessState,
    /// Task IDs belonging to this process.
    pub tasks: Vec<TaskId>,
    /// Parent process ID (0 = kernel).
    pub parent: ProcessId,
    /// Isolation domain for this process.
    pub isolation: IsolationDomain,
    /// Capabilities granted to this process.
    pub capabilities: CapabilityTable,
    /// Physical address of the level-4 page table for this process.
    pub page_table: Option<PhysAddr>,
    /// True when page_table frame is uniquely owned by this process.
    pub owns_page_table: bool,
    /// User virtual ranges owned by this process (start, len).
    pub user_regions: Vec<(u64, u64)>,
}

impl Process {
    pub fn new(name: &str, parent: ProcessId) -> Self {
        Process {
            pid: ProcessId::new(),
            name: String::from(name),
            state: ProcessState::Created,
            tasks: Vec::new(),
            parent,
            isolation: IsolationDomain::user(0), // Default to basic user isolation
            capabilities: CapabilityTable::new(),
            page_table: None,
            owns_page_table: false,
            user_regions: Vec::new(),
        }
    }

    pub fn add_task(&mut self, task_id: TaskId) {
        self.tasks.push(task_id);
    }
}

lazy_static! {
    static ref PROCESS_TABLE: Mutex<ProcessTable> = Mutex::new(ProcessTable::new());
}

/// Simple process table.
pub struct ProcessTable {
    pub processes: Vec<Process>,
}

impl ProcessTable {
    pub const fn new() -> Self {
        ProcessTable {
            processes: Vec::new(),
        }
    }

    pub fn add(&mut self, process: Process) {
        self.processes.push(process);
    }

    pub fn find(&self, pid: ProcessId) -> Option<&Process> {
        self.processes.iter().find(|p| p.pid == pid)
    }

    pub fn find_mut(&mut self, pid: ProcessId) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }
}

/// Result of best-effort task resource cleanup during reap.
#[derive(Debug, Clone, Copy)]
pub struct TaskCleanupReport {
    pub closed_fds: usize,
    pub released_process_links: usize,
    pub released_ipc_links: usize,
    pub released_user_regions: usize,
    pub released_page_tables: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct CleanupTelemetry {
    pub cleanup_events: u64,
    pub cleanup_fds_total: u64,
    pub cleanup_links_total: u64,
    pub cleanup_regions_total: u64,
    pub cleanup_page_tables_total: u64,
}

/// Best-effort cleanup hook for task-owned resources.
/// This is intentionally lightweight for now and can be extended with
/// address-space frame reclamation once per-task memory ownership is tracked.
pub fn cleanup_task_resources(task_id: TaskId) -> TaskCleanupReport {
    let closed_fds = crate::fs::vfs::close_task_fds(task_id.0);
    let (released_process_links, released_user_regions, released_page_tables) =
        cleanup_process_links_for_task(task_id);
    let report = TaskCleanupReport {
        closed_fds,
        released_process_links,
        released_ipc_links: 0,
        released_user_regions,
        released_page_tables,
    };
    CLEANUP_EVENTS.fetch_add(1, Ordering::Relaxed);
    CLEANUP_FDS_TOTAL.fetch_add(report.closed_fds as u64, Ordering::Relaxed);
    CLEANUP_LINKS_TOTAL.fetch_add(report.released_process_links as u64, Ordering::Relaxed);
    CLEANUP_REGIONS_TOTAL.fetch_add(report.released_user_regions as u64, Ordering::Relaxed);
    CLEANUP_PT_TOTAL.fetch_add(report.released_page_tables as u64, Ordering::Relaxed);
    report
}

pub fn register_process(mut process: Process) -> ProcessId {
    let pid = process.pid;
    process.state = ProcessState::Running;
    PROCESS_TABLE.lock().add(process);
    pid
}

pub fn attach_task_to_process(pid: ProcessId, task_id: TaskId) {
    if let Some(p) = PROCESS_TABLE.lock().find_mut(pid) {
        p.add_task(task_id);
    }
}

pub fn mark_process_zombie_if_empty(pid: ProcessId) {
    if let Some(p) = PROCESS_TABLE.lock().find_mut(pid) {
        if p.tasks.is_empty() {
            p.state = ProcessState::Zombie;
        }
    }
}

fn cleanup_process_links_for_task(task_id: TaskId) -> (usize, usize, usize) {
    let mut links = 0usize;
    let mut regions = 0usize;
    let mut page_tables = 0usize;
    let mut table = PROCESS_TABLE.lock();
    for p in table.processes.iter_mut() {
        let before = p.tasks.len();
        p.tasks.retain(|id| *id != task_id);
        if p.tasks.len() != before {
            links += before - p.tasks.len();
            if p.tasks.is_empty() {
                p.state = ProcessState::Zombie;
                regions += p.user_regions.len();
                p.user_regions.clear();
                if let Some(pt) = p.page_table.take() {
                    if p.owns_page_table {
                        crate::memory::frame_allocator::deallocate_frame(
                            PhysFrame::containing_address(pt),
                        );
                    }
                    page_tables += 1;
                }
            }
        }
    }
    (links, regions, page_tables)
}

/// Replace the page-table pointer for the process that owns `tid`.
/// Called by exec() after the new address space is ready.
pub fn update_task_page_table(tid: TaskId, new_cr3: PhysAddr, owns: bool) {
    let mut table = PROCESS_TABLE.lock();
    for p in table.processes.iter_mut() {
        if p.tasks.contains(&tid) {
            p.page_table = Some(new_cr3);
            p.owns_page_table = owns;
            p.user_regions.clear();
            break;
        }
    }
}

pub fn cleanup_telemetry() -> CleanupTelemetry {
    CleanupTelemetry {
        cleanup_events: CLEANUP_EVENTS.load(Ordering::Relaxed),
        cleanup_fds_total: CLEANUP_FDS_TOTAL.load(Ordering::Relaxed),
        cleanup_links_total: CLEANUP_LINKS_TOTAL.load(Ordering::Relaxed),
        cleanup_regions_total: CLEANUP_REGIONS_TOTAL.load(Ordering::Relaxed),
        cleanup_page_tables_total: CLEANUP_PT_TOTAL.load(Ordering::Relaxed),
    }
}
