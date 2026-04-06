# Contributing to Florynx OS

## Getting Started

1. Install Rust nightly: `rustup install nightly`
2. Install bootimage: `cargo install bootimage`
3. Install QEMU: Download from https://www.qemu.org/
4. Clone the repository and run: `cd florynx-kernel && cargo run`

## Code Style

- Follow standard Rust formatting (`cargo fmt`)
- Add documentation comments to all public items
- Minimize `unsafe` — add safety comments where needed
- Keep modules focused and under 300 lines

## Subsystem Guide

| Subsystem | Status | Location |
|-----------|--------|----------|
| Boot | ✅ Complete | `src/main.rs`, `src/lib.rs` |
| CPU Setup | ✅ Complete | `src/arch/x86_64/` |
| Memory | ✅ Complete | `src/memory/` |
| Scheduler | ✅ Complete | `src/process/` |
| Drivers | ✅ Complete | `src/drivers/` |
| Syscalls | ✅ Complete | `src/syscall/` |
| IPC | ✅ Structs | `src/ipc/` |
| Security | ✅ Structs | `src/security/` |
| Filesystem | ✅ Structs | `src/fs/` |
| GUI | 🔲 Stub | `src/gui/` |
| Runtime | 🔲 Stub | `src/runtime/` |
| WinCompat | 🔲 Stub | `src/wincompat/` |
