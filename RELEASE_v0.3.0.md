# 🚀 Florynx-OS v0.3.0 — Production-Level Exception Handling

**Release Date**: April 7, 2026  
**Codename**: "Sentinel"

---

## 🎯 What's New

### Production-Level Exception Handling System

Florynx-OS v0.3.0 introduces **enterprise-grade exception handling** comparable to Linux and Windows kernels. Every kernel exception now provides detailed diagnostics, CPU state dumps, and automatic stack traces for debugging.

### ✨ Key Features

**🔍 Enhanced Exception Diagnostics**
- Complete CPU state dump (all 16 general-purpose registers + control registers)
- Automatic stack trace with return address unwinding
- Detailed error analysis for every exception type
- Beautiful formatted output for serial debugging

**🛡️ 9 Exception Handlers Implemented**
- ✅ Divide Error (Vector 0) — Division by zero detection
- ✅ Debug (Vector 1) — Hardware debug support
- ✅ Breakpoint (Vector 3) — Software breakpoint handling
- ✅ Invalid Opcode (Vector 6) — Illegal instruction detection
- ✅ Double Fault (Vector 8) — Critical fault recovery with IST
- ✅ Stack Segment Fault (Vector 12) — Stack overflow detection
- ✅ General Protection Fault (Vector 13) — Privilege violation analysis
- ✅ Page Fault (Vector 14) — Memory access violation with detailed breakdown
- ✅ Alignment Check (Vector 17) — Unaligned memory access detection

**📊 Page Fault Analysis**
Every page fault now shows:
- Faulting virtual address (from CR2 register)
- Whether page is present or not mapped
- Read vs write access violation
- User mode vs kernel mode access
- Instruction fetch violations
- Reserved bit violations

**🔬 Stack Trace Walker**
- Automatic stack unwinding using RBP chain
- Displays up to 10 stack frames
- Shows return addresses for debugging
- Validates kernel address ranges
- Prevents infinite loops on corrupted stacks

---

## 🐛 Bug Fixes (v0.2.5)

- ✅ Fixed window drag causing full screen reload
- ✅ Fixed keyboard backspace not working (HandleControl setting)
- ✅ Made dock icons fully functional and clickable
- ✅ Each dock icon now creates specific application windows

---

## 📦 What's Included

### Core Kernel Features
- **Memory Management**: Paging, heap allocator (4 MiB), frame allocator
- **Interrupt System**: PIC, IDT, 9 exception handlers, IRQ routing
- **Exception Handling**: Production-level diagnostics and debugging
- **Assembly Utilities**: Context switching, I/O ports, GDT/IDT management

### GUI System
- **Window Manager**: Drag & drop, focus management, z-order
- **Rendering**: 60 FPS with dirty-rect optimization, 1024x768 framebuffer
- **Widgets**: Button, TextInput, Panel components
- **Applications**: Text editor with multi-line editing, line numbers, toolbar
- **Dock**: Functional dock with 5 clickable icons (Files, Terminal, Settings, Monitor, Notes)

### Input/Output Drivers
- **Keyboard**: PS/2 driver with full character mapping, modifier key support
- **Mouse**: PS/2 driver with smooth cursor tracking
- **Display**: BGA framebuffer driver, VGA text mode fallback
- **Serial**: UART 16550 for debugging output

### Performance
- **Frame Limiter**: 60 FPS cap reduces CPU usage by ~70%
- **Partial Redraw**: Dirty rectangle optimization
- **Background Cache**: Cached gradient background (2.25 MiB)

---

## 🎨 Visual Improvements

- Enhanced panic handler with framebuffer output (visible in GUI mode)
- Beautiful formatted exception output with box-drawing characters
- Color-coded error messages (red for critical errors)
- Professional boot banner

---

## 🔧 Technical Details

**Architecture**: x86_64  
**Boot Protocol**: Multiboot2 via bootloader crate  
**Memory Model**: Higher-half kernel (0xFFFF800000000000)  
**Heap Size**: 4 MiB  
**Screen Resolution**: 1024x768 @ 32-bit color  
**Timer Frequency**: 200 Hz (PIT)  
**Frame Rate**: 60 FPS (capped)  

**Exception Handling**:
- IST (Interrupt Stack Table) for double fault handler
- Separate stack for critical exceptions
- Non-blocking locks in panic state
- Full register state preservation

---

## 📚 Documentation

- **Architecture Diagram**: Complete system architecture in `docs/architecture-diagram.md`
- **Architecture Guide**: Detailed design in `docs/architecture.md`
- **Evolution Log**: All changes tracked in `docs/evolutions.md`
- **README**: Quick start guide in `README.md`

---

## 🚀 Getting Started

### Prerequisites
- Rust nightly toolchain
- QEMU x86_64 emulator
- Git

### Build & Run
```bash
# Clone the repository
git clone https://github.com/Florynx-labs/florynx-os.git
cd florynx-os/florynx-kernel

# Build the kernel
cargo +nightly build

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=target/x86_64-florynx/debug/bootimage-florynx-kernel.bin -serial stdio
```

### Testing Exception Handlers
To test the exception handling system, you can trigger exceptions:
- **Page Fault**: Access unmapped memory
- **Divide Error**: Divide by zero
- **Invalid Opcode**: Execute invalid instruction
- **GPF**: Access invalid segment

All exceptions will display detailed diagnostics in the serial output.

---

## 🎯 What's Next — v0.4.0 Roadmap

### Phase 2: File System (In Progress)
- ✅ VFS (Virtual File System) abstraction layer
- ✅ Ramdisk filesystem driver
- ✅ File operations (open, read, write, close, seek)
- ✅ Directory operations (readdir, mkdir, rmdir, stat)

### Phase 3: Process Management
- Task scheduler (round-robin or priority-based)
- Process creation and termination
- Context switching using existing `switch_to` function
- Process states (running, ready, blocked, zombie)

### Phase 4: System Calls & User Space
- System call interface (int 0x80 or syscall instruction)
- User/kernel mode separation
- Memory protection
- Basic syscalls (exit, fork, exec, read, write)

---

## 🤝 Contributing

Florynx-OS is an open-source project. Contributions are welcome!

**Areas for contribution**:
- Additional filesystem drivers (FAT32, ext2)
- Network stack (TCP/IP)
- USB support
- ACPI implementation
- SMP (multi-core) support

---

## 📄 License

This project is licensed under the MIT License.

---

## 🙏 Acknowledgments

- **Rust Community**: For the amazing `x86_64` and `bootloader` crates
- **OSDev Community**: For extensive documentation and support
- **Phil Opp**: For the "Writing an OS in Rust" blog series inspiration

---

## 📊 Statistics

- **Total Lines of Code**: ~15,000
- **Kernel Size**: ~500 KB
- **Boot Time**: < 1 second
- **Memory Usage**: ~6 MiB (kernel + heap + framebuffer)
- **Exception Handlers**: 9
- **GUI Windows**: Unlimited (memory-limited)
- **Supported Input Devices**: Keyboard, Mouse
- **Supported Output**: Framebuffer, Serial, VGA Text

---

**Download**: [GitHub Releases](https://github.com/Florynx-labs/florynx-os/releases/tag/v0.3.0)  
**Documentation**: [docs/](https://github.com/Florynx-labs/florynx-os/tree/main/docs)  
**Issues**: [GitHub Issues](https://github.com/Florynx-labs/florynx-os/issues)

---

<p align="center">
  <strong>Florynx-OS v0.3.0 "Sentinel"</strong><br>
  Production-Level Exception Handling • 60 FPS GUI • Full Input Support
</p>
