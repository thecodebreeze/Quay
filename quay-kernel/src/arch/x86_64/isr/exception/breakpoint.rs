use log::debug;
use x86_64::structures::idt::InterruptStackFrame;

/// Handles the breakpoint interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame) {
    debug!(
        r#"
        ### BREAKPOINT ###
            Stack Frame:
                {:#?}
        "#,
        stack_frame
    );
}
