use log::debug;
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::PrivilegeLevel;

/// Handles the divide error interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame) {
    let is_userspace = stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3;

    if is_userspace {
        debug!(
            r#"
            ### DIVIDE ERROR ###
                Instruction Pointer: {:#X}
                Stack Frame:
                    {:#?}
            "#,
            stack_frame.instruction_pointer.as_u64(),
            stack_frame
        );

        // TODO: Terminate the offending process/thread.
        // TODO: Force the CPU to schedule a different process so we don't return to the faulting instruction.
        // TODO: Manipulate the stack frame to trigger a SIGFPE.
        // TODO: hijack the instruction_pointer as well to trigger a task-cleanup on the OS.
    } else {
        panic!(
            r#"
            ### DIVIDE ERROR ###
                Instruction Pointer: {:#X}
                Stack Frame:
                    {:#?}
            "#,
            stack_frame.instruction_pointer.as_u64(),
            stack_frame
        )
    }
}
