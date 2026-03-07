use thiserror_no_std::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PmmError {
    #[error("Out of physical memory: requested order {0}")]
    OutOfMemory(usize),

    #[error("Requested allocation order {0} exceeds maximum supported order {1}")]
    OrderTooLarge(usize, usize),

    #[error("Attempted to free unaligned memory physical address {0:#X} for order {1}")]
    UnalignedFree(u64, usize),
}
