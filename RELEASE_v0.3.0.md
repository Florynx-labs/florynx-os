# Florynx-OS v0.3.0 "Sentinel"

**Release Date**: April 8, 2026  
**Codename**: "Sentinel"  
**Features**: 108 across 7 phases

---

## What's New in v0.3.0

### Kernel / Userland Separation
- **florynx-kernel/**: Ring 0 kernel — memory, scheduling, drivers, syscalls, security
- **florynx-userland/**: KDE Plasma-inspired desktop shell, built-in apps, system services
- **shared/**: Syscall ABI definitions and shared types (Rect, Color, GuiEvent, WindowParams)

### KDE Plasma-Inspired Desktop Shell
- **Bottom panel**: [App Menu] [Taskbar] [System Tray + Clock]
- **Kickoff-style app launcher** with categories (Favorites, System, Utilities)
- **Task manager** with active window highlighting (accent indicator)
- **System tray** with clock display
- **Notification daemon** (top-right popups with timeout)
- **Session manager** (active, locked, logging out)
- **Breeze Bioluminescent theme** — KDE Breeze Dark adapted with Florynx cyan/green accents

### 3 Default Wallpapers
| # | Name | Style |
|---|------|-------|
| 1 | Bioluminescent Crystals | Green & cyan crystal formations |
| 2 | Flowing Waves | Abstract teal energy waves |
| 3 | Nebula | Green-cyan cosmic nebula |

### Animation Engine (Phase 7)
- `lerp()`, `ease_out()`, `ease_in_out()` interpolation
- `AnimatedPos` — smooth window drag (speed=0.35)
- `AnimatedOpacity` — window fade-in on creation
- `AnimatedScale` — dock hover magnification (1.25x)
- Per-frame `tick_animations()` in compositor loop

### Per-Window Compositor Buffers
- Each window has its own offscreen `Vec<u8>` buffer
- `dirty` flag: only re-render windows whose content changed
- Focus changes mark old/new active windows dirty automatically

### Double-Buffered Rendering (Phase 6)
- RAM back buffer → VRAM flush (no direct MMIO writes)
- 32-rect dirty engine with merge (no full-screen redraws)
- Cursor: back-buffer draw + flush ~14x20px region only
- Background cache (~2.3 MiB) prevents gradient re-render

### IPC Event Bus
- System-wide pub/sub with 64-entry ring buffers
- 32 concurrent subscriptions
- EventType: Task, Window, Input, File, Channel events
- SystemEvent: type tag + 3 u64 args + timestamp

### Security (Phase 5)
- 18 capability flags (CAP_NET, CAP_FS, CAP_GUI, CAP_DEVICE, etc.)
- CapabilitySet with presets (MINIMAL, STANDARD, PRIVILEGED, ROOT)
- Audit log with timestamps
- DevFS: /dev/null, /dev/zero, /dev/serial0

### Exception Handling
- 9 handlers with full CPU state dump + stack trace
- Page fault analysis (present, read/write, user/kernel, fetch, reserved)
- IST-backed double fault handler
- Stack trace walker (up to 10 frames)

---

## Built-in Applications (Userland)

| App | Inspired by | Status |
|-----|-------------|--------|
| Files | KDE Dolphin | Scaffold |
| Terminal | KDE Konsole | Scaffold |
| Settings | KDE System Settings | Scaffold |
| System Monitor | KSysGuard | Scaffold |
| Text Editor | KDE Kate | Scaffold |

---

## Technical Details

| Parameter | Value |
|-----------|-------|
| Architecture | x86_64 |
| Language | Rust nightly (`#![no_std]`) |
| Heap Size | 16 MiB |
| Resolution | 1024x768 @ 32bpp |
| Timer | PIT at 200 Hz |
| Syscalls | 11 (POSIX) + 7 (GUI) + 3 (IPC) |
| Capabilities | 18 bitflags |
| Dirty rects | 32, with merge |
| Wallpapers | 3 default |
| Total features | 108 |

---

## Build & Run

```bash
git clone https://github.com/Florynx-labs/florynx-os.git
cd florynx-os/florynx-kernel
cargo +nightly bootimage
qemu-system-x86_64 \
  -drive format=raw,file=target/x86_64-florynx/debug/bootimage-florynx-kernel.bin \
  -serial stdio -m 128
```

---

## Phase History

| Phase | Name | Features |
|-------|------|----------|
| 1 | Kernel Stabilization | GDT, IDT, PIC, memory, drivers |
| 2 | GUI Desktop | Compositor, windows, dock, icons |
| 3 | Exception Handling | 9 handlers, stack trace, diagnostics |
| 4 | VFS + Scheduler | Ramdisk, tmpfs, devfs, round-robin |
| 5 | Security + Syscalls | 18 capabilities, audit, 11 syscalls |
| 6 | GUI Performance | Double-buffer, dirty-rect, cursor opt |
| 7 | Animation + Compositor | LERP engine, per-window buffers, IPC bus |

---

## What's Next — v0.4.0

- **Ring 3 userland**: True user-mode process execution
- **WASM runtime**: Sandboxed application bytecode interpreter
- **SMP**: Multi-core scheduler
- **Network stack**: TCP/IP, sockets
- **FAT32**: Persistent filesystem

---

## Documentation

- **[Architecture](docs/architecture.md)** — Full system architecture with Mermaid diagrams
- **[Evolutions](docs/evolutions.md)** — All 108 features tracked across 7 phases
- **[README](README.md)** — Quick start guide

---

**Download**: [GitHub Releases](https://github.com/Florynx-labs/florynx-os/releases/tag/v0.3.0)  
**Documentation**: [docs/](https://github.com/Florynx-labs/florynx-os/tree/main/docs)  
**Issues**: [GitHub Issues](https://github.com/Florynx-labs/florynx-os/issues)

---

<p align="center">
  <strong>Florynx-OS v0.3.0 "Sentinel"</strong><br>
  KDE Plasma-Inspired Shell • 108 Features • Built from Scratch in Rust
</p>
