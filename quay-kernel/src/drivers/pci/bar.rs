/// PCIe Base Address Register (BAR).
#[derive(Debug)]
pub enum PciBar {
    /// Memory-Mapped I/O (32-bit)
    Memory32 { address: u32, prefetchable: bool },

    /// Memory-Mapped I/O (64-bit)
    Memory64 { address: u64, prefetchable: bool },

    /// I/O Ports
    LegacyIo { port: u32 },
}
