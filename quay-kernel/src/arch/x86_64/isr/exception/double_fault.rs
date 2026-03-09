use x86_64::structures::idt::InterruptStackFrame;

/// Handles the double fault interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!(
        "\n\
        ================================================================\n\
        |                        DOUBLE FAULT                          |\n\
        ================================================================\n\
        | Instruction Pointer: {:<39?} |\n\
        | Stack Pointer:       {:<39?} |\n\
        | CPU Flags:           {:<39?} |\n\
        | Code Segment:        {:<39?} |\n\
        | Stack Segment:       {:<39?} |\n\
        ================================================================",
        stack_frame.instruction_pointer,
        stack_frame.stack_pointer,
        stack_frame.cpu_flags,
        stack_frame.code_segment,
        stack_frame.stack_segment
    );
}
