# FlorynxOS Roadmap — From Hybrid Kernel to Everyday OS

This document maps out the gap between the current FlorynxOS foundation and a fully usable, high-performance hybrid operating system. 
By combining the stability of a micro-kernel with the raw speed of monolithic drivers (for gaming and heavy I/O), FlorynxOS aims for uncompromising UI fluidity.

---

## Current State (v0.4.x)

| Subsystem | Status |
|-----------|--------|
| **Bootloader** | bootimage crate, BIOS boot, 1024×768 BGA framebuffer |
| **Kernel arch** | x86_64, GDT/TSS, IDT with CPU exceptions + PIC IRQs |
| **Memory** | Physical frame allocator, 4-level paging, kernel heap (linked list) |
| **Interrupts** | PIC 8259 cascade, PIT timer (200 Hz), PS/2 keyboard + mouse |
| **Processes** | Preemptive round-robin scheduler, Ring 3 user tasks, per-process page table |
| **Syscalls** | int 0x80 ABI, ~20 syscalls (open/read/write/close/sleep/gui/exit…) |
| **Filesystem** | In-memory VFS + ramdisk + devfs. No persistent storage. |
| **IPC** | Channel-based message passing, event bus |
| **Security** | Capability sets, user-pointer validation, supervisor-only kernel mappings |
| **GUI** | Kernel-mode compositor, wallpaper, windows, dock, menu bar, cursor |
| **Userland** | Minimal Ring 3 stub (sleep loop). No shell, no libc. |

---

## Phase 1 — Core Kernel Maturity

### 1.1 Real Context Switching
- [x] Save/restore full register state (GPRs, FPU/SSE) on task switch
- [x] Per-task kernel stacks (16 KiB, allocated at spawn)
- [ ] Proper idle task integration (event pump in idle context, not main loop)

### 1.2 Virtual Memory Per-Process
- [x] Demand paging (page fault → allocate frame on first access)
- [x] Guard pages for stack overflow detection
- [ ] Copy-on-write fork
- [ ] User-space `mmap` / `munmap` syscalls

### 1.3 Signals & Process Control
- [x] POSIX-style signal delivery (SIGTERM, SIGKILL, SIGHUP, SIGINT)
- [x] `waitpid` (specific PID) + `wait` (any child)
- [x] `fork()` + `execve()` syscalls
- [ ] Process groups and sessions

### 1.4 SMP (Symmetric Multi-Processing)
- [ ] APIC initialization (LAPIC + I/O APIC, replace PIC)
- [ ] AP (Application Processor) bootstrap
- [ ] Per-CPU scheduler run queues
- [ ] Spinlock/ticket-lock audit for multi-core safety

### 1.5 Timekeeping
- [x] RTC (CMOS) read for wall-clock time + date
- [x] `clock_gettime` / `gettimeofday` syscalls
- [ ] HPET or TSC-based high-resolution timer

---

## Phase 2 — Storage & Persistent Filesystem

### 2.1 Block Device Drivers
- [ ] ATA/AHCI (SATA) driver — reads/writes sectors
- [ ] NVMe driver (for modern SSDs)
- [x] Virtio-blk driver (QEMU/KVM) — PCI discovery + polled I/O

### 2.2 Persistent Filesystem
- [x] FAT32 read-only (short names, cluster chain, VFS mount)
- [ ] FAT32 write support
- [ ] ext2 read/write (native Linux-style FS)
- [ ] Partition table parsing (MBR / GPT)
- [ ] Mount / unmount syscalls

### 2.3 Bootloader Upgrade
- [ ] UEFI boot support (replace BIOS-only bootimage)
- [ ] GRUB or Limine integration
- [ ] Boot from real disk (not just ramdisk)

---

## Phase 3 — Networking

### 3.1 NIC Drivers
- [ ] Virtio-net driver (QEMU/KVM)
- [ ] Intel e1000 / e1000e driver (common in VMs and laptops)
- [ ] RTL8139 (simple, well-documented)

### 3.2 Network Stack
- [ ] Ethernet frame handling (L2)
- [ ] ARP
- [ ] IPv4 (and eventually IPv6)
- [ ] ICMP (ping)
- [ ] UDP
- [ ] TCP (connection state machine, retransmit, congestion control)
- [ ] DHCP client (auto-configure IP)
- [ ] DNS resolver

### 3.3 Userland Network API
- [ ] BSD socket syscalls (`socket`, `bind`, `listen`, `accept`, `connect`, `send`, `recv`)
- [ ] `getaddrinfo` stub in libc

---

## Phase 4 — Userland & Standard Library

### 4.1 C Runtime / libc Port
- [ ] Minimal custom libc (or port musl/newlib)
- [ ] `printf`, `malloc`/`free`, `string.h` basics
- [ ] POSIX file I/O wrappers (`fopen`, `fread`, `fwrite`)
- [ ] `errno` and signal handling

### 4.2 ELF Loader
- [ ] Parse ELF64 headers
- [ ] Load `.text`, `.data`, `.bss` segments into user address space
- [ ] Dynamic linking support (`ld.so`, `.so` loading)
- [x] `execve` syscall (flat-binary loader at 0x400000)

### 4.3 Shell
- [x] Interactive command-line shell (`florsh`)
- [ ] PATH lookup, piping (`|`), redirection (`>`, `<`)
- [ ] Job control (fg, bg, Ctrl-C)
- [x] Built-in commands: `cd`, `ls`, `cat`, `echo`, `exit`, `clear`

### 4.4 Core Utilities
- [ ] `ls`, `cat`, `cp`, `mv`, `rm`, `mkdir`, `rmdir`
- [ ] `grep`, `find`, `wc`
- [ ] `ps`, `kill`, `top`
- [ ] `mount`, `umount`, `df`
- [ ] Text editor (`nano`-like or custom)

---

## Phase 5 — GUI Maturity

### 5.1 Font Rendering
- [ ] TrueType / OpenType font parser (via `ab_glyph` or `fontdue` no_std for high-quality TTF/OTF)
- [ ] Variable-width font support
- [ ] Anti-aliased text rendering
- [ ] Font size selection

### 5.2 Compositor Improvements
- [x] Per-pixel alpha blending mechanics (`Color::rgba` / Frosty blur logic)
- [x] Phase 4 GUI Integrations (Interactive Shell, Menubar, Dynamic Dock)
- [ ] Render API Optimizations (Hardware blitting, partial compositing upgrades in Ring 0 for Speed)
- [ ] TrueType Font (TTF) Rendering Engine & Advanced Typography (via `ab_glyph`)
- [ ] PNG Decoder for High-Res Custom Icons & Wallpapers (via `png` crate no_std)

### 5.3 Widget Toolkit
- [ ] Scrollbar, checkbox, radio button, dropdown
- [ ] List view, tree view, tab control
- [ ] Dialog boxes (file open/save, message box)
- [ ] Clipboard (copy/paste between windows)
- [ ] Drag-and-drop

### 5.4 Desktop Applications
- [ ] File manager (browse VFS, open files)
- [ ] Terminal emulator (PTY-backed, runs shell)
- [ ] Text editor (syntax highlighting)
- [ ] System monitor (CPU/memory/disk usage)
- [ ] Settings panel (display, input, network config)
- [ ] Image viewer
- [ ] Calculator
- [ ] Web browser (very long-term, requires networking + rendering engine)

---

## Phase 6 — Hardware Compatibility

### 6.1 USB
- [ ] xHCI (USB 3.x) host controller driver
- [ ] USB HID (keyboard, mouse) — replace PS/2
- [ ] USB mass storage (thumb drives)
- [ ] USB hub support

### 6.2 Audio
- [ ] Intel HDA driver
- [ ] AC97 fallback
- [ ] Mixer / volume control
- [ ] Audio API for userland

### 6.3 Graphics
- [ ] VESA/GOP framebuffer (for real hardware, not just BGA)
- [ ] Hardware-accelerated 2D (if available)
- [ ] Resolution switching at runtime
- [ ] Multi-GPU awareness

### 6.4 Input
- [ ] USB HID gamepad support
- [ ] Touchpad gestures (multi-finger)
- [ ] On-screen keyboard (accessibility)

---

## Phase 7 — Security & Stability

- [ ] ASLR (Address Space Layout Randomization)
- [ ] Stack canaries / SSP
- [ ] NX bit enforcement on all data pages
- [ ] Seccomp-like syscall filtering
- [ ] User authentication (login screen, user accounts, passwords)
- [ ] File permissions (owner/group/other, rwx bits)
- [ ] Encrypted storage support (LUKS or similar)
- [ ] Kernel module signing (if modules are added)

---

## Phase 8 — Distribution & Installation

- [ ] ISO image builder (bootable from USB/CD)
- [ ] Installer wizard (partition, format, copy, configure bootloader)
- [ ] Package manager (`fpkg` or similar — install/update/remove software)
- [ ] Package repository hosting
- [ ] Automatic updates
- [ ] First-boot setup wizard (language, timezone, user account)
- [ ] Documentation: user manual, developer guide, API reference

---

## Priority Summary

| Priority | Items |
|----------|-------|
| **P0 — Now** | ELF64 loader (proper segments), shell (`florsh`), idle-task event pump, FAT32 write |
| **P1 — Near** | UEFI boot, networking, libc, core utils, font rendering |
| **P2 — Mid** | USB, audio, SMP, window resize, terminal emulator, settings |
| **P3 — Later** | ASLR, installer, package manager, multi-monitor, web browser |

---

*Last updated: FlorynxOS v0.4.8 — Pivot to Hybrid Kernel Architecture (High Performance + TrueType / PNG Pipeline)*
