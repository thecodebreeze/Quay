use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};

/// Handles the page fault interrupt event.
pub extern "x86-interrupt" fn handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    let accessed_address = Cr2::read();
    panic!(
        "\n\
        ================================================================\n\
        |                         PAGE FAULT                           |\n\
        ================================================================\n\
        | Accessed Address:    {:<39?} |\n\
        | Error Code:          {:<39?} |\n\
        |--------------------------------------------------------------|\n\
        | Instruction Pointer: {:<39?} |\n\
        | Stack Pointer:       {:<39?} |\n\
        | CPU Flags:           {:<39?} |\n\
        ================================================================",
        accessed_address,
        error_code,
        stack_frame.instruction_pointer,
        stack_frame.stack_pointer,
        stack_frame.cpu_flags
    );
}
