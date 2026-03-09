/// A generic interface for a high-precision hardware timer.
pub trait SystemTimer: Send + Sync {
    /// Number of nanoseconds elapsed since boot.
    fn nanos_since_boot(&self) -> u64;

    /// Busy-wait the CPU for a given number of microseconds.
    ///
    /// MUST NOT yield to the scheduler.
    fn stall_us(&self, microseconds: u64);

    /// Sleeps for a given number of milliseconds.
    fn sleep_ms(&self, milliseconds: u64);
}
