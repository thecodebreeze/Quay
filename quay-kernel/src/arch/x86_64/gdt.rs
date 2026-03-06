//! Global Descriptor Table.
//!
//! Creates a GDT that contains a proper TSS with a single IST index for double faults. If the
//! system ever has a stack overflow, it doesn't trigger a chain overflow which would cause a
//! triple fault.
//!
//! Since the system is going to use Paging, we only need minimal segments. But even as Limine gives
//! us a GDT, the system can't really trust it.

use lazy_static::lazy_static;
use x86_64::VirtAddr;
use x86_64::instructions::tables::load_tss;
use x86_64::registers::segmentation::{CS, DS, ES, FS, GS, SS, Segment};
use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable, SegmentSelector};
use x86_64::structures::tss::TaskStateSegment;

/// Assign the Double Fault stack to index zero in the IST.
pub const GDT_DOUBLE_FAULT_IST_INDEX: u16 = 0;

/// Special stack space for double faults.
const GDT_IST_DOUBLE_FAULT_STACK_SIZE: usize = 65536;

lazy_static! {
    pub static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();

        // Create and save the new selectors.
        let kernel_code_selector = gdt.append(Descriptor::kernel_code_segment());
        let kernel_data_selector = gdt.append(Descriptor::kernel_data_segment());
        let tss_selector = gdt.append(Descriptor::tss_segment(&TSS));

        // This order is actually important, it's critical for `sysret` as it expects this kind of
        // alignment.
        let user_data_selector = gdt.append(Descriptor::user_data_segment());
        let user_code_selector = gdt.append(Descriptor::user_code_segment());

        (gdt, Selectors {
            kernel_code_selector,
            kernel_data_selector,
            tss_selector,
            user_data_selector,
            user_code_selector,
        })
    };
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();

        // Set the IST with the Double Fault extra stack.
        tss.interrupt_stack_table[GDT_DOUBLE_FAULT_IST_INDEX as usize] = {
            // This must be mutable static, since the system is going to write to it. We reserve
            // this space right inside the kernel binary.
            static mut STACK: [u8; GDT_IST_DOUBLE_FAULT_STACK_SIZE] = [0u8; GDT_IST_DOUBLE_FAULT_STACK_SIZE];

            // Calculate the top of the stack (since in the x86_64 architecture it grows downwards).
            #[allow(unused_unsafe)]
            let stack_ptr = unsafe { core::ptr::addr_of!(STACK) };

            // The kernel is loaded in the higher half of the virtual address space, so when
            // referencing statics and other variables, Virtual Addresses must be used.
            let stack_start = VirtAddr::from_ptr(stack_ptr);

            // Return the end of the stack.
            stack_start + GDT_IST_DOUBLE_FAULT_STACK_SIZE as u64
        };

        tss
    };
}

/// Helper struct to hold the memory offsets (selectors) of the GDT entries.
pub struct Selectors {
    kernel_code_selector: SegmentSelector,
    kernel_data_selector: SegmentSelector,
    tss_selector: SegmentSelector,
    user_data_selector: SegmentSelector,
    user_code_selector: SegmentSelector,
}

/// Loads the GDT and updates the CPU segment registers.
pub fn load_global_descriptor_table() {
    // Load the GDT into the CPU.
    GDT.0.load();

    // The CPU won't use the new GDT until we reload the segment registers.
    unsafe {
        CS::set_reg(GDT.1.kernel_code_selector);
        DS::set_reg(GDT.1.kernel_data_selector);
        ES::set_reg(GDT.1.kernel_data_selector);

        // Clear FS and GS as well. Their actual addresses will be set via MSRs.
        FS::set_reg(GDT.1.kernel_data_selector);
        GS::set_reg(GDT.1.kernel_data_selector);

        // Load the Stack Pointer and the TSS.
        SS::set_reg(GDT.1.kernel_data_selector);
        load_tss(GDT.1.tss_selector);
    }
}
