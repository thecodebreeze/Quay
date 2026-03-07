//! The Virtual Memory Manager (VMM) implementation with MMIO utilities.
//!
//! Quay VMM implementation is platform-agnostic and works by interacting with the also agnostic
//! Physical Memory Manager (PMM).
//!
//! The VMM implementation allows for allocation of 4KiB, 2MiB, and 1GiB pages. But other mappings
//! are possible if the PMM supports it.
//!
//! Platform-dependent code for the VMM can be found in the arch module.

pub mod error;
pub mod map_flags;
pub mod page_size;

use crate::sys::memory::pmm::DEFAULT_PAGE_SIZE;
pub use error::VmmError;
use log::error;
pub use map_flags::MapFlags;
pub use page_size::PageSize;

/// Core trait that every architecture (x86, ARM, RISC-V) must implement.
pub trait VirtualMapper<'a> {
    /// Maps a physical address to a virtual address with specific flags and size.
    ///
    /// # Safety
    ///
    /// The caller must ensure the physical address is valid and not already mapped in a conflicting
    /// way.
    unsafe fn map_page(
        &mut self,
        virt_addr: u64,
        phys_addr: u64,
        size: PageSize,
        flags: MapFlags,
    ) -> Result<(), VmmError>;

    /// Unmaps a virtual address, returning the physical address it was pointing to.
    fn unmap_page(&mut self, virt_addr: u64) -> Result<u64, VmmError>;
}

/// Maps a block of physical MMIO memory into the HHDM, automatically using 2MiB pages if the
/// alignment and size permit it.
pub fn map_mmio_range(
    mapper: &mut dyn VirtualMapper,
    phys_addr: u64,
    size_bytes: u64,
    hhdm_offset: u64,
) -> Result<(), VmmError> {
    let mut current_phys_addr = phys_addr;
    let end_phys_addr = phys_addr.saturating_add(size_bytes);

    // MMIO must always bypass the CPU cache.
    let flags = MapFlags::PRESENT | MapFlags::WRITABLE | MapFlags::NO_CACHE;

    while current_phys_addr < end_phys_addr {
        let virt_addr = current_phys_addr.saturating_add(hhdm_offset);

        // Determine if we can use a 2MiB Big Page by subtracting the current address from the last
        // address of the range.
        //
        // TODO: Implement 1GiB Huge Page.
        let is_2mb_aligned = current_phys_addr.is_multiple_of(0x200000);
        let has_2mb_remaining = end_phys_addr
            .saturating_sub(current_phys_addr)
            .gt(&0x200000);

        // Calculate the step and page sizes depending on the detection.
        let (step_size, page_size) = if is_2mb_aligned && has_2mb_remaining {
            (0x200000, PageSize::Size2MiB)
        } else {
            (DEFAULT_PAGE_SIZE, PageSize::Size4KiB)
        };

        unsafe {
            mapper
                .map_page(virt_addr, current_phys_addr, page_size, flags)
                .inspect_err(|error| {
                    error!(
                        "Failed to map MMIO range: {:X?} -> {:X?} ({:#?})",
                        virt_addr, current_phys_addr, error
                    );
                })?;
        }

        current_phys_addr = current_phys_addr.saturating_add(step_size as u64);
    }

    Ok(())
}
