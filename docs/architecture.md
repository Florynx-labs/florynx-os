# Florynx Kernel Architecture

## Overview

Florynx Kernel is a modular x86_64 kernel written in Rust. It follows a layered architecture with clean subsystem separation.

## Boot Sequence

1. **Bootloader** (`bootloader` crate) loads the kernel binary, sets up long mode (64-bit), identity-maps physical memory, and jumps to `kernel_main`.
2. **GDT** — Global Descriptor Table with kernel code/data segments and TSS.
3. **IDT** — Interrupt Descriptor Table with CPU exception handlers and IRQ handlers.
4. **PIC** — 8259 Programmable Interrupt Controller initialized with offsets 32/40.
5. **PIT** — Programmable Interval Timer at 100 Hz.
6. **Interrupts enabled.**
7. **Paging** — OffsetPageTable created from active CR3 page table.
8. **Frame Allocator** — Boot info frame allocator scanning usable memory regions.
9. **Heap** — 1 MiB linked-list heap at virtual address 0x4444_4444_0000.
10. **Scheduler** — Demo tasks registered and run.
11. **Idle loop** — HLT loop waiting for interrupts.

## Memory Map

| Region | Address | Size |
|--------|---------|------|
| VGA Buffer | 0xB8000 | 4000 bytes |
| Kernel Heap | 0x4444_4444_0000 | 1 MiB |
| Physical memory offset | Bootloader-defined | All RAM |

## Interrupt Vectors

| Vector | Source | Handler |
|--------|--------|---------|
| 3 | Breakpoint | Debug logging |
| 8 | Double Fault | Panic (uses IST) |
| 14 | Page Fault | Panic with CR2 |
| 13 | GPF | Panic |
| 32 (IRQ0) | Timer | PIT tick + scheduler |
| 33 (IRQ1) | Keyboard | PS/2 scancode decode |

## Subsystem Dependencies

```
main.rs
  └── lib.rs (init)
        ├── arch/x86_64/gdt
        ├── arch/x86_64/idt
        │     └── interrupts/pic
        │     └── drivers/timer/pit
        │     └── drivers/input/keyboard
        ├── memory/paging
        ├── memory/frame_allocator
        ├── memory/heap
        └── process/scheduler
              └── process/task
```
