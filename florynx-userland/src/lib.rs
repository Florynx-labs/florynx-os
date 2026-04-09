//! Florynx Userland — KDE Plasma-inspired desktop environment.
//!
//! This crate contains the GUI shell, applications, and system services
//! that run on top of the Florynx kernel via the syscall interface.
//!
//! # Architecture
//!
//! ```text
//! florynx-userland/
//! ├── gui/            Desktop shell (KDE Plasma-style)
//! │   ├── shell.rs    Desktop shell compositor
//! │   ├── panel.rs    Top/bottom panel (like KDE Plasma panel)
//! │   ├── app_menu.rs Application launcher (like Kickoff)
//! │   ├── taskbar.rs  Task manager + window list
//! │   ├── systray.rs  System tray (clock, volume, network)
//! │   ├── wallpaper.rs Wallpaper manager
//! │   └── theme.rs    Breeze-inspired dark theme
//! ├── apps/           Built-in applications
//! │   ├── files.rs    File manager (Dolphin-like)
//! │   ├── terminal.rs Terminal emulator (Konsole-like)
//! │   ├── settings.rs System settings
//! │   ├── monitor.rs  System monitor
//! │   └── editor.rs   Text editor (Kate-like)
//! └── system/         System services
//!     ├── session.rs  Session manager
//!     └── notif.rs    Notification daemon
//! ```

#![no_std]

extern crate alloc;

pub mod gui;
pub mod apps;
pub mod system;
pub mod syscall;
