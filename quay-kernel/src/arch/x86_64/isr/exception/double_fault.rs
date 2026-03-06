use x86_64::structures::idt::InterruptStackFrame;

/// Handles the double fault interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame, _error_code: u64) -> ! {
    panic!(
        r#"
        ### DOUBLE FAULT ###
            Instruction Pointer: {:#X}
            Stack Frame:
                {:#?}
        "#,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame
    );
}
