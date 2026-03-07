use bitflags::bitflags;

bitflags! {
    /// Architecture-agnostic memory mapping flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MapFlags: u64 {
        const PRESENT   = 1 << 0;
        const WRITABLE  = 1 << 1;
        const USER      = 1 << 2; // Accessible from Ring 3
        const NO_EXECUTE= 1 << 3; // Prevent code execution (NX bit)
        const NO_CACHE  = 1 << 4; // Crucial for MMIO!
    }
}
