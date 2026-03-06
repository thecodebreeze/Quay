//! Platform-specific code for the x86_64 architecture.
//!
//! This module defines key, core functionality for working with x86_64 such as the GDT, IDT,
//! APIC, Timers, and safe wrappers around x86_64 specific assembly instructions.

pub mod gdt;
pub mod idt;
pub mod pic;
mod isr;
pub mod apic;
pub mod timer;
pub mod cpu;