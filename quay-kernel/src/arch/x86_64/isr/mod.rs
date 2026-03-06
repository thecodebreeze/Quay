use x86_64::structures::idt::InterruptStackFrame;

pub mod exception;
pub mod hardware;

/// This ISR does nothing. Really. It's meant to be a NOOP.
pub(super) extern "x86-interrupt" fn no_op_isr(_stack_frame: InterruptStackFrame) {}
