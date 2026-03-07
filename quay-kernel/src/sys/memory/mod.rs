//! Memory Manager implementations and utilities.
//!
//! Our Page Allocator uses a Buddy Allocator design to support big pages. It's also faster than our
//! previous Bitmap Allocator.

pub mod phys_memory_manager;
pub mod virt_memory_manager;
