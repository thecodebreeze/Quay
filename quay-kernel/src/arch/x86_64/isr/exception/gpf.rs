use bitflags::bitflags;
use x86_64::structures::idt::InterruptStackFrame;

bitflags! {
    #[repr(transparent)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct GeneralProtectionFaultErrorCode: u64 {
        /// If set, the exception occurred during delivery of an external event
        /// (like a hardware interrupt). If clear, it was caused by normal execution.
        const EXTERNAL = 1 << 0;

        /// If set, the index portion refers to a gate descriptor in the
        /// Interrupt Descriptor Table (IDT).
        const IDT = 1 << 1;

        /// Table Indicator. Only relevant if IDT is clear (0).
        /// If set, the index refers to the Local Descriptor Table (LDT).
        /// If clear, the index refers to the Global Descriptor Table (GDT).
        const TABLE_INDICATOR = 1 << 2;

        /// A bitmask covering bits 3 through 15. We use this internally
        /// to mask out the selector index.
        const SELECTOR_INDEX_MASK = 0xFFF8;
    }
}

impl GeneralProtectionFaultErrorCode {
    /// Returns true if the CPU could not trace the fault to a specific descriptor.
    ///
    /// This generally means the fault was caused by:
    /// * Executing privileged instructions from user mode.
    /// * Writing to a reserved bit in a control register.
    /// * Accessing a memory address that violates segment limits.
    /// * Executing an instruction requiring alignment on unaligned memory.
    pub fn is_untraceable(&self) -> bool {
        self.is_empty()
    }

    /// Extracts the specific 13-bit index of the descriptor that caused the fault.
    pub fn selector_index(&self) -> u16 {
        ((self.bits() & Self::SELECTOR_INDEX_MASK.bits()) >> 3) as u16
    }

    /// A helper function for your panic handler to easily print which table failed.
    pub fn table_name(&self) -> &'static str {
        if self.contains(Self::IDT) {
            "IDT"
        } else if self.contains(Self::TABLE_INDICATOR) {
            "LDT"
        } else {
            "GDT"
        }
    }
}

/// Handles the general protection fault interrupt event.
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame, error_code: u64) {
    let error_code = GeneralProtectionFaultErrorCode::from_bits_truncate(error_code);
    if error_code.is_untraceable() {
        // TODO: This is recoverable and must return SIGSEGV or SIGILL and then kill the task.
        panic!(
            r#"
            ### GENERAL PROTECTION FAULT ###
                Error Code: Untraceable (0x0)
                Instruction Pointer: {:#X}
                Stack Frame:
                    {:#?}
            "#,
            stack_frame.instruction_pointer.as_u64(),
            stack_frame
        )
    } else {
        panic!(
            r#"
            ### GENERAL PROTECTION FAULT ###
                Error Code: {:#06X} (External: {})
                Faulting Table: {}
                Faulting Selector Index: {}
                Instruction Pointer: {}
                Stack Frame:
                    {:#?}
            "#,
            error_code.bits(),
            error_code.contains(GeneralProtectionFaultErrorCode::EXTERNAL),
            error_code.table_name(),
            error_code.selector_index(),
            stack_frame.instruction_pointer.as_u64(),
            stack_frame
        )
    }
}
