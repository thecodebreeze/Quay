use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

/// Initialize a new OffsetPageTable.
pub fn init_mapper<'a>(hhdm_offset: VirtAddr) -> OffsetPageTable<'a> {
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
