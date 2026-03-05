use crate::HHDM_REQUEST;
use crate::memory::pmm::GlobalBitmapPMM;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{
    Mapper, OffsetPageTable, Page, PageTable, PageTableFlags, PhysFrame, Size4KiB, mapper,
};
use x86_64::{PhysAddr, VirtAddr};

lazy_static! {
    pub static ref VMM_MAPPER: Mutex<OffsetPageTable<'static>> = {
        let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM to be present");
        let hhdm_offset_virtual_address = VirtAddr::new(hhdm_response.offset());

        let mapper = init_mapper(hhdm_offset_virtual_address);
        Mutex::new(mapper)
    };
}

/// Maps a contiguous block of physical MMIO memory into the HHDM.
pub fn map_mmio_range(phys_addr: u64, size_bytes: u64, hhdm_offset: u64) {
    let mut mapper = crate::memory::vmm::VMM_MAPPER.lock();
    let mut frame_allocator = GlobalBitmapPMM;

    let start_frame = phys_addr / 4096;
    let end_frame = (phys_addr + size_bytes + 4095) / 4096;

    // MMIO memory MUST have the NO_CACHE flag, so the CPU doesn't cache hardware state!
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;

    for frame_idx in start_frame..end_frame {
        let phys = frame_idx * 4096;
        let virt = phys + hhdm_offset;

        let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(phys));
        let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virt));

        unsafe {
            match mapper.map_to(page, frame, flags, &mut frame_allocator) {
                Ok(flusher) => flusher.flush(),
                Err(MapToError::PageAlreadyMapped(_)) => {
                    // If already mapped, forcefully update the flags to ensure NO_CACHE is set
                    mapper
                        .update_flags(page, flags)
                        .expect("Failed to update flags")
                        .flush();
                }
                Err(e) => panic!("Failed to map MMIO: {:?}", e),
            }
        }
    }
}

/// Maps a single physical MMIO memory address into the HHDM.
pub fn map_mmio(hhdm_offset: u64, physical_address: u64) {
    let mut frame_allocator = GlobalBitmapPMM;
    let mut mapper = VMM_MAPPER.lock();

    let virtual_address = physical_address + hhdm_offset;
    let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(physical_address));
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virtual_address));

    // MMIO must be explicitly marked as NO_CACHE so the CPU talks directly to the hardware.
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
    unsafe {
        match mapper.map_to(page, frame, flags, &mut frame_allocator) {
            Ok(flusher) => flusher.flush(),
            Err(mapper::MapToError::PageAlreadyMapped(_)) => {
                // If Limine mapped it as Read-Only, forcefully override the flags to Writable!
                mapper
                    .update_flags(page, flags)
                    .expect("Failed to update page flags!")
                    .flush();
            }
            Err(error) => panic!("Failed to map MMIO: {:?}", error),
        }
    }
}

/// Initialize a new OffsetPageTable.
fn init_mapper<'a>(hhdm_offset: VirtAddr) -> OffsetPageTable<'a> {
    // Read the active Level4 page table from the CR3 Register.
    let (level_4_table_frame, _) = Cr3::read();

    // Calculate the virtual address of the Level 4 table using the HHDM.
    let physical_address = level_4_table_frame.start_address();
    let virtual_address = hhdm_offset + physical_address.as_u64();

    // Get a mutable reference to the table.
    let page_table_ptr: *mut PageTable = virtual_address.as_mut_ptr();
    let level_4_table = unsafe { &mut *page_table_ptr };

    // Return the mapper abstraction.
    unsafe { OffsetPageTable::new(level_4_table, hhdm_offset) }
}
