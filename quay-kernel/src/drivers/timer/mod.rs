//! This module implement timer drivers for different platforms.

#[cfg(target_arch = "x86_64")]
pub mod tsc;
