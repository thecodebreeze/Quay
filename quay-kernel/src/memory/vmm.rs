use crate::HHDM_REQUEST;
use lazy_static::lazy_static;
use spin::Mutex;
use x86_64::VirtAddr;
use x86_64::registers::control::Cr3;
use x86_64::structures::paging::{OffsetPageTable, PageTable};

lazy_static! {
    pub static ref VMM_MAPPER: Mutex<OffsetPageTable<'static>> = {
        let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM to be present");
        let hhdm_offset_virtual_address = VirtAddr::new(hhdm_response.offset());

        let mapper = init_mapper(hhdm_offset_virtual_address);
        Mutex::new(mapper)
    };
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
