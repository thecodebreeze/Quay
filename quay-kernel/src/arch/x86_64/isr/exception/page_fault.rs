use x86_64::registers::control::Cr2;
use x86_64::structures::idt::{InterruptStackFrame, PageFaultErrorCode};
use x86_64::VirtAddr;

/// Handles the page fault interrupt event.
pub extern "x86-interrupt" fn handler(
    stack_frame: InterruptStackFrame,
    error_code: PageFaultErrorCode,
) {
    // TODO: this is totally recoverable and implementation of the cases must be done.
    panic!(
        r#"
        ### PAGE FAULT ###
            Accessed Address: {:X}
            Error Code: {:?}
            Stack Frame:
                {:#?}
        "#,
        Cr2::read().unwrap_or(VirtAddr::new(0)),
        error_code,
        stack_frame
    )
}
