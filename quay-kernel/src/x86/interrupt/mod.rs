pub mod apic;
pub mod timer;

use crate::x86::gdt::DOUBLE_FAULT_IST_INDEX;
use core::sync::atomic::{AtomicUsize, Ordering};
use lazy_static::lazy_static;
use log::{debug, error};
use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};

lazy_static! {
    static ref IDT: InterruptDescriptorTable = unsafe {
        let mut idt = InterruptDescriptorTable::new();
        idt.breakpoint.set_handler_fn(breakpoint_handler);
        idt.double_fault
            .set_handler_fn(double_fault_handler)
            .set_stack_index(DOUBLE_FAULT_IST_INDEX);
        idt.divide_error.set_handler_fn(divide_handler);
        idt.general_protection_fault
            .set_handler_fn(general_protection_fault_handler);
        idt.page_fault.set_handler_fn(page_fault_handler);

        idt[32].set_handler_fn(apic_timer_interrupt_handler);
        idt[34].set_handler_fn(apic_error_interrupt_handler);
        idt[255].set_handler_fn(spurious_interrupt_handler);

        idt[KEYBOARD_INTERRUPT_ID].set_handler_fn(apic::keyboard_interrupt_handler);

        idt
    };
}

/// Loads the IDT into the CPU using the `LIDT` assembly instruction.
pub fn init_idt() {
    IDT.load();
}

/// Returns the number of milliseconds since the kernel enabled interrupts.
pub fn uptime_ms() -> usize {
    TICKS.load(Ordering::Relaxed)
}

/// A blocking sleep function.
/// Note: This busy-waits, which is fine for now, but in the future
/// this will tell the scheduler to put the current thread to sleep.
pub fn sleep_ms(ms: usize) {
    let start_time = uptime_ms();
    while uptime_ms() < start_time + ms {
        // Optimization: Don't just spin, halt the CPU until the next tick!
        x86_64::instructions::hlt();
    }
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

extern "x86-interrupt" fn general_protection_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: u64,
) {
    panic!(
        "EXCEPTION: GENERAL PROTECTION FAULT (Error Code: {})\n{:#?}",
        error_code, stack_frame
    );
}

use crate::x86::interrupt::apic::{
    KEYBOARD_INTERRUPT_ID, TICKS, apic_error_interrupt_handler, apic_timer_interrupt_handler,
    spurious_interrupt_handler,
};
use x86_64::structures::idt::PageFaultErrorCode;

extern "x86-interrupt" fn page_fault_handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    use x86_64::registers::control::Cr2;
    panic!(
        "EXCEPTION: PAGE FAULT\nAccessed Address: {:?}\nError Code: {:?}\n{:#?}",
        Cr2::read(),
        error_code,
        stack_frame
    );
}
