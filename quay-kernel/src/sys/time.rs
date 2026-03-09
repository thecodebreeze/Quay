//! Architecture-agnostic time subsystem.
//!
//! This module acts as the central timekeeper for the kernel. Hardware-specific drivers (like the
//! x86_64 TSC) are initialized during boot and registered here.
//!
//! The rest of the kernel (including ACPI and schedulers) can then safely request the time without
//! needing to know the underlying hardware details.

use crate::hal::timer::SystemTimer;
use alloc::boxed::Box;
use log::info;
use spin::Once;

/// Global system timer instance.
static ACTIVE_TIMER: Once<Box<dyn SystemTimer>> = Once::new();

/// Register a hardware timer as the official system timekeeper.
///
/// This should be called exactly once during the hardware initialization phase of boot.
pub fn register_timer(timer: Box<dyn SystemTimer>) {
    ACTIVE_TIMER.call_once(|| timer);
    info!("System timer registered globally.");
}

/// Returns true if the system timer is ready to be read.
#[inline(always)]
pub fn is_ready() -> bool {
    ACTIVE_TIMER.is_completed()
}

/// Returns the number of nanoseconds elapsed since the system booted.
///
/// # Panic
///
/// Panics if called before the system timer is registered.
pub fn nanos_since_boot() -> u64 {
    if let Some(timer) = ACTIVE_TIMER.get() {
        timer.nanos_since_boot()
    } else {
        panic!("CRITICAL: nanos_since_boot() called before the system timer is registered!");
    }
}

/// Stalls the CPU for a given number of microseconds (1,000,000 us = 1s).
///
/// This is a hardware busy-wait and will **not** yield the CPU to the scheduler.
///
/// It is primarily used by hardware drivers (like ACPI) that need very short, precise delays to
/// wait for hardware registers to update.
///
/// # Panic
///
/// Panics if called before the system timer is registered.
pub fn stall_us(microseconds: u64) {
    if let Some(timer) = ACTIVE_TIMER.get() {
        timer.stall_us(microseconds);
    } else {
        panic!(
            "CRITICAL: stall_us({}) called before the system timer is registered!",
            microseconds
        );
    }
}

/// Sleeps for a given number of milliseconds.
///
/// # Panic
///
/// Panics if called before the system timer is registered.
pub fn sleep_ms(milliseconds: u64) {
    if let Some(timer) = ACTIVE_TIMER.get() {
        timer.sleep_ms(milliseconds);
    } else {
        panic!(
            "CRITICAL: sleep_ms({}) called before the system timer is registered!",
            milliseconds
        );
    }
}
