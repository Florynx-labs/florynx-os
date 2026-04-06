# Florynx Kernel Design

## Design Principles

1. **Modularity** — Each subsystem is a separate Rust module with clear interfaces.
2. **Safety** — Use safe Rust wherever possible; minimize `unsafe` blocks.
3. **Clarity** — Every module and function is documented with comments.
4. **Scalability** — Stub modules (GUI, runtime, WinCompat) are architected for future expansion.

## Unsafe Usage

Unsafe code is confined to:
- Hardware port I/O (PIC, PIT, keyboard)
- Page table manipulation (CR3 access)
- GDT/TSS loading
- VGA buffer memory-mapped I/O
- Frame allocator initialization (trusting bootloader memory map)
- Heap allocator initialization

All unsafe blocks have safety comments explaining why they are necessary.

## Key Design Decisions

### Bootloader
Using the `bootloader` crate (v0.9) instead of a custom bootloader. This gives us:
- Proper long mode (64-bit) setup
- Physical memory identity mapping
- Boot information (memory map, framebuffer info)

### Memory
- **Bump allocator** for physical frames (simple, no deallocation — suitable for early boot).
- **Linked-list allocator** for the kernel heap (supports arbitrary alloc/dealloc).
- **OffsetPageTable** with bootloader-provided physical memory offset.

### Scheduling
- **Cooperative round-robin** — tasks run their entry function and yield.
- **Timer integration** — PIT tick notifies the scheduler (infrastructure for preemptive scheduling).

### Interrupt Handling
- **Legacy PIC** (8259) for now — APIC stub ready for SMP upgrade.
- Interrupts disabled during lock acquisition (deadlock prevention).
