use crate::x86::gdt::DOUBLE_FAULT_IST_INDEX;
use lazy_static::lazy_static;
use log::{debug, error, info};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = unsafe {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        idt.divide_error.set_handler_fn(divide_handler);
        idt
    };
}

/// Loads the IDT into the CPU using the `LIDT` assembly instruction.
pub fn init_idt() {
    IDT.load();
}

/// Exception handler for breakpoints.
extern "x86-interrupt" fn breakpoint_handler(stack_frame: InterruptStackFrame) {
    debug!("EXCEPTION: BREAKPOINT\n{:#?}", stack_frame);
}

/// Handler for a Double Fault.
///
/// A double fault happens when the CPU fails to invoke an exception handler.
extern "x86-interrupt" fn double_fault_handler(
    stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    error!("EXCEPTION: DOUBLE FAULT\n{:#?}", stack_frame);
    panic!("PARRY THIS YOU FILTHY CASUAL!");
}

/// Handler for errors related to numerical divisions.
extern "x86-interrupt" fn divide_handler(stack_frame: InterruptStackFrame) {
    error!("EXCEPTION: DIVIDE BY ZERO\n{:#?}", stack_frame);
}
