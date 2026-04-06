<p align="center">
  <img src="florynxlogo.png" alt="FlorynxOS" width="420" />
</p>

<p align="center">
  <strong>A bioluminescent desktop OS built from scratch in Rust.</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/arch-x86__64-blue" alt="arch" />
  <img src="https://img.shields.io/badge/lang-Rust%20nightly-orange" alt="lang" />
  <img src="https://img.shields.io/badge/version-0.2-cyan" alt="version" />
</p>

---

## Overview

FlorynxOS is a modern x86_64 operating system kernel with a graphical desktop shell, written entirely in Rust with no external runtime. It boots from bare metal into a dark, premium GUI with draggable windows, a floating dock, and a mouse cursor — all rendered on a raw framebuffer.

## Architecture

```
florynx-kernel/src/
├── arch/x86_64/       GDT, IDT, PIC, PIT, CPU detection
├── core/              Kernel core, panic handler, logging macros
├── drivers/
│   ├── display/       BGA graphics, framebuffer, VGA text
│   ├── input/         PS/2 keyboard + mouse (with timeouts)
│   ├── serial/        UART 16550 debug output
│   └── timer/         PIT at 100 Hz
├── memory/            Paging, O(1) frame allocator, heap (1 MiB)
├── gui/
│   ├── renderer.rs    Drawing primitives, 8x8 font, cursor
│   ├── theme.rs       Bioluminescent color palette (PRD)
│   ├── desktop.rs     Compositor, window manager, cached background
│   ├── window.rs      Draggable windows with titlebar buttons
│   ├── dock.rs        Floating bottom dock with icons
│   ├── icons.rs       16x16 + 8x8 bitmap icons
│   ├── event.rs       Mouse event dispatch
│   └── console.rs     Framebuffer text console
├── process/           Task scheduler (round-robin)
├── interrupts/        PIC, interrupt dispatch
├── syscall/           Syscall table
├── ipc/               Message passing
├── fs/                VFS stubs
├── security/          Capability stubs
└── main.rs            Boot sequence (6 phases)
```

## Prerequisites

- **Rust nightly** (auto-configured via `rust-toolchain.toml`)
- **QEMU** (`qemu-system-x86_64`)
- **bootimage**:
  ```
  cargo install bootimage
  ```

## Build

```bash
cd florynx-kernel
cargo +nightly build
```

## Run in QEMU

```bash
cargo +nightly bootimage
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-florynx/debug/bootimage-florynx-kernel.bin \
  -serial stdio
```

## Features

### Kernel
- Bootable x86_64 bare-metal kernel
- GDT with TSS (double-fault IST stack)
- IDT with exception + hardware IRQ handlers
- PIC-based interrupts, PIT timer at 100 Hz
- Virtual memory paging with identity-mapped physical memory
- O(1) bump frame allocator (region-tracking)
- Kernel heap — 1 MiB linked-list allocator
- PS/2 keyboard + mouse with timeout-safe init
- Serial debug output (UART 16550, COM1)
- Round-robin task scheduler

### GUI Desktop Shell
- BGA framebuffer (1024×768, 32bpp)
- Dark bioluminescent theme (cyan/mint accents from logo)
- Gradient background with noise + vignette (cached for performance)
- Draggable windows with rounded corners and shadow
- Traffic-light titlebar buttons (close/minimize/maximize)
- Floating dock with bitmap icons and hover highlight
- Hardware cursor with dirty-rect save/restore
- Window manager with z-ordering and focus tracking
- Mouse event dispatch (click, drag, hover)

## Boot Sequence

```
Phase 1  GDT → IDT → PIC + PIT         (interrupts disabled)
Phase 2  Paging → Frame alloc → Heap
Phase 3  BGA framebuffer → Console
Phase 4  Enable interrupts
Phase 5  Post-init → Desktop launch
Phase 6  hlt_loop with GUI redraw
```

### Serial Output
```
[kernel] core init complete (interrupts still disabled)
=========================================
  Florynx Kernel v0.2 — Booting...
=========================================
[boot] heap initialized
[boot] interrupts ENABLED
[desktop] GUI initialized (1024x768)
========================================
  Kernel OK — System stable.
========================================
[kernel] entering GUI hlt_loop
```

## License

This project is part of the Florynx OS initiative by [Florynx Labs](https://github.com/Florynx-labs).
