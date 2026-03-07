use crate::sys::memory::pmm::PmmError;
use thiserror_no_std::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum VmmError {
    #[error("Virtual address {0:#X} is already mapped to another physical frame")]
    AlreadyMapped(u64),

    #[error("Virtual address {0:#X} is not mapped")]
    NotMapped(u64),

    #[error("The requested page size is not supported by the hardware")]
    UnsupportedPageSize,

    #[error("Page table allocation failed due to PMM error: {0}")]
    PageTableAllocationFailed(#[from] PmmError),

    #[error("Hardware mapping failure: {0}")]
    HardwareFault(&'static str),
}
