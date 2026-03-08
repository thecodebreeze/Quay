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

pub use error::VmmError;
pub use map_flags::MapFlags;
pub use page_size::PageSize;

use crate::arch::x86_64::vmm::X86Mapper;
use crate::sys::memory::pmm::DEFAULT_PAGE_SIZE;
use log::error;
use spin::{Mutex, MutexGuard, Once};
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

/// Global VMM instance.
pub static VMM: Once<Mutex<X86Mapper<'static>>> = Once::new();

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

pub fn initialize(hhdm_offset: u64) {
    // Read the CR3 register to find the physical address of the Level4 Page Table.
    let (level_4_table_frame, _) = Cr3::read();
    let phys_addr = level_4_table_frame.start_address().as_u64();

    // Convert that physical address to a virtual address using the HHDM offset.
    let virt_addr = phys_addr.saturating_add(hhdm_offset);

    // Cast the virtual address into a mutable Rust reference to the PageTable struct.
    // Safety: This memory is valid because the CPU is actively using it right now!
    let level_4_table: &'static mut PageTable = unsafe { &mut *(virt_addr as *mut PageTable) };

    // Create the x86_64 crate's OffsetPageTable mapper.
    let mapper = unsafe { OffsetPageTable::new(level_4_table, VirtAddr::new(hhdm_offset)) };

    // Wrap it in the custom X86Mapper and store it in the global static.
    let x86_mapper = X86Mapper::new(mapper);

    VMM.call_once(|| Mutex::new(x86_mapper));

    log::info!("VMM initialized using CR3 frame at {:#X}", phys_addr);
}

/// A clean helper function to grab the VMM lock from anywhere in the kernel.
pub fn get_vmm() -> MutexGuard<'static, X86Mapper<'static>> {
    VMM.get().expect("VMM has not been initialized yet!").lock()
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
