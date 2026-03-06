//! Utilities for initializing, synchronizing, and using system timers.
//!
//! Currently using the LAPIC Timer as the main time keeping and the HPET for synchronization. The
//! time module is designed so only CORE0 can update the system ticks, making it the official
//! timekeeper.

use core::hint::spin_loop;
use core::sync::atomic;
use core::sync::atomic::AtomicU64;

/// The global system tick counter.
static SYSTEM_TICKS: AtomicU64 = AtomicU64::new(0);

/// The frequency of our APIC timer in Hz.
///
/// Our target is 1 tick per millisecond. This is used not only for the cooperative multitasking
/// but also for sleep implementation later down the line.
pub const TICKS_PER_SECOND: u64 = 1000;

/// Called by the BSP (Core 0) inside its timer interrupt handler.
pub fn increment_system_ticks() {
    SYSTEM_TICKS.fetch_add(1, atomic::Ordering::Relaxed);
}

/// Returns the number of ticks since the kernel booted.
pub fn get_system_ticks() -> u64 {
    SYSTEM_TICKS.load(atomic::Ordering::Relaxed)
}

/// Returns the system uptime in milliseconds.
pub fn kernel_uptime_ms() -> u64 {
    get_system_ticks()
        .saturating_mul(TICKS_PER_SECOND)
        .saturating_div(1000)
}

/// A hardware-level busy-wait.
///
/// This is strictly for drivers needing millisecond hardware delays.
pub fn spin_delay_ms(milliseconds: u64) {
    let start_ticks = get_system_ticks();
    let ticks_to_wait = milliseconds
        .saturating_mul(TICKS_PER_SECOND)
        .saturating_div(1000);
    let target_ticks = start_ticks.saturating_add(ticks_to_wait);

    // Run the busy-wait loop. Signaling the CPU that it might optimize itself by doing some other
    // work.
    while get_system_ticks() < target_ticks {
        // In x86_64 this emits a `pause` assembly instruction.
        //
        // It stops the CPU from actually iterating this loop of death. Since it allows other
        // threads to do work while the kernel internal clock does not tick.
        spin_loop()
    }
}
