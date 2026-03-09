use log::debug;
use x86_64::PrivilegeLevel;
use x86_64::structures::idt::InterruptStackFrame;

/// Handles the divide error interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame) {
    let is_userspace = stack_frame.code_segment.rpl() == PrivilegeLevel::Ring3;

    if is_userspace {
        debug!(
            "\n\
            ================================================================\n\
            |                       DIVIDE BY ZERO                         |\n\
            ================================================================\n\
            | Instruction Pointer: {:<39?} |\n\
            | Stack Pointer:       {:<39?} |\n\
            | CPU Flags:           {:<39?} |\n\
            ================================================================",
            stack_frame.instruction_pointer, stack_frame.stack_pointer, stack_frame.cpu_flags
        );

        // TODO: Terminate the offending process/thread.
        // TODO: Force the CPU to schedule a different process so we don't return to the faulting instruction.
        // TODO: Manipulate the stack frame to trigger a SIGFPE.
        // TODO: hijack the instruction_pointer as well to trigger a task-cleanup on the OS.
    } else {
        panic!(
            "\n\
            ================================================================\n\
            |                       DIVIDE BY ZERO                         |\n\
            ================================================================\n\
            | Instruction Pointer: {:<39?} |\n\
            | Stack Pointer:       {:<39?} |\n\
            | CPU Flags:           {:<39?} |\n\
            ================================================================",
            stack_frame.instruction_pointer, stack_frame.stack_pointer, stack_frame.cpu_flags
        );
    }
}
