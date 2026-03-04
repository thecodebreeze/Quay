use core::ptr;
use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, DS, ES, SS, Segment};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

/// We assign the Double Fault stack to index 0 in the IST.
pub const DOUBLE_FAULT_IST_INDEX: u16 = 0;

lazy_static! {
    /// Create the TSS with a single IST index for our double faults, so that if we ever have a
    /// stack overflow. At the very least, we don't triple fault.
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // Set up the emergency stack for double faults.
        tss.interrupt_stack_table[DOUBLE_FAULT_IST_INDEX as usize] = {
            // 32 KiB Stack should suffice, hopefully.
            const STACK_SIZE: usize = 4096 * 8;

            // We use mutable static to allocate this memory right inside the kernel binary.
            static mut STACK: [u8; STACK_SIZE] = [0; STACK_SIZE];

            // Calculate the top of the stack (since in x86 it grows downwards).
            #[allow(unused_unsafe)]
            let stack_ptr = unsafe { ptr::addr_of!(STACK) };
            let stack_start = VirtAddr::from_ptr(stack_ptr);

            // Return the end of the stack.
            stack_start + STACK_SIZE as u64
        };

        tss
    };
}

lazy_static! {
    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        // Add our segments to the GDT.
        let code_selector = gdt.append(Descriptor::kernel_code_segment());
        let data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));

        (gdt, Selectors {
            code_selector,
            data_selector,
            tss_selector
        })
    };
}

/// Helper struct to hold the memory offsets (selectors) of our GDT entries.
struct Selectors {
    code_selector: SegmentSelector,
    data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
}

/// Loads the GDT and updates the CPU segment registers.
pub fn init_gdt() {
    // Load the GDT into the CPU.
    GDT.0.load();

    // The CPU won't use it until we update the segment registers.
    unsafe {
        CS::set_reg(GDT.1.code_selector);
        DS::set_reg(GDT.1.data_selector);
        ES::set_reg(GDT.1.data_selector);
        SS::set_reg(GDT.1.data_selector);

        // Tell the CPU to load our TSS as well.
        load_tss(GDT.1.tss_selector);
    }
}
