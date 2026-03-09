//! Platform-dependent code.
//!
//! This module aggregates all code that is specific for a single platform like x86_64, ARM64 or
//! RISC-V.

#[cfg(target_arch = "x86_64")]
pub mod x86_64;

#[cfg(target_arch = "x86_64")]
pub use x86_64 as target;

#[cfg(target_arch = "aarch64")]
pub mod aarch64;

#[cfg(target_arch = "aarch64")]
pub use aarch64 as target;

#[cfg(target_arch = "riscv64")]
pub mod rv64;

#[cfg(target_arch = "riscv64")]
pub use rv64 as target;
