//! Memory Manager implementations and utilities.
//!
//! This module contains three types of memory managers.
//!
//! ## Physical Memory Manager (PMM)
//!
//! Responsible to interface with the actual hardware to allocate pages in physical memory.
//!
//! ## Virtual Memory Manager (VMM)
//!
//! Responsible to interface with the PMM through an abstraction layer. The trait provided must be
//! implemented for each platform that Quay targets.
//!
//! ## Kernel Heap Manager (KHM)
//!
//! Responsible for managing the heap memory of the kernel space. This is a simple interface on top
//! of [talc]. It provides the global allocator for the kernel.

pub mod khm;
pub mod pmm;
pub mod vmm;
