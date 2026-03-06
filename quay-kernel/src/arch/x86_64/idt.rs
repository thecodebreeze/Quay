//! Interrupt Descriptor Table.
//!
//! Creates a IDT that is used by the CPU to handle interrupts. Not every single one is set yet, but
//! they will be added as development progresses.
//!
//! For a baseline, the included interrupts are:
//! * Breakpoint
//! * Double Fault
//! * Divide Error
//! * General Protection Fault
//! * Page Fault
//! * APIC Timer
//! * APIC Error

use crate::arch::x86_64::isr;
use crate::platform::gdt::DOUBLE_FAULT_IST_INDEX;
use lazy_static::lazy_static;
use x86_64::structures::idt::InterruptDescriptorTable;

/// Define the APIC Error interrupt vector index.
pub const APIC_ERROR_VECTOR_INDEX: u8 = 32;

/// Define the APIC Timer interrupt vector index.
pub const APIC_TIMER_VECTOR_INDEX: u8 = 33;

/// Define the APIC Spurious interrupt vector index.
pub const APIC_SPURIOUS_VECTOR_INDEX: u8 = 255;

lazy_static! {
    pub static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();

        // CPU Exceptions are hard-wired to be in the indexes 0 to 31.
        idt.divide_error.set_handler_fn(isr::exception::divide::handler);
        idt.breakpoint.set_handler_fn(isr::exception::breakpoint::handler);
        let double_fault = idt.double_fault.set_handler_fn(isr::exception::double_fault::handler);
        idt.page_fault.set_handler_fn(isr::exception::page_fault::handler);
        idt.general_protection_fault.set_handler_fn(isr::exception::gpf::handler);

        // Set the stack index for the double fault handler.
        unsafe {
            double_fault.set_stack_index(DOUBLE_FAULT_IST_INDEX);
        }

        // Hardware interrupts can be mapped on demand starting from index 32.
        idt[APIC_ERROR_VECTOR_INDEX].set_handler_fn(isr::hardware::apic_error::handler);
        idt[APIC_TIMER_VECTOR_INDEX].set_handler_fn(isr::hardware::apic_timer::handler);
        idt[APIC_SPURIOUS_VECTOR_INDEX].set_handler_fn(isr::no_op_isr);

        idt
    };
}

/// Load the IDT into the CPU.
///
/// Although this can allow calling more than once since it is protected by a lazy_static block,
/// it's not recommended to do so.
pub fn load_interrupt_descriptor_table() {
    IDT.load();
}
