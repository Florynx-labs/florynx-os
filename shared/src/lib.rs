//! Florynx Shared Types — kernel ↔ userland interface definitions.
//! These types are used by both the kernel syscall layer and the userland libraries.

#![no_std]

pub mod syscall_abi;
pub mod types;
