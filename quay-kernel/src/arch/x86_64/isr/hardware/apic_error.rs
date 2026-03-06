use crate::arch::x86_64::apic::{get_lapic_address, LAPIC_ESR_REGISTER};
use crate::arch::x86_64::cpu::CpuLocalData;
use bitflags::bitflags;
use core::ops::Add;
use log::debug;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::idt::InterruptStackFrame;

bitflags! {
    /// Flags used to decode APIC Error status codes.
    #[repr(transparent)]
    #[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone, Copy)]
    pub struct ApicErrorFlags: u32 {
        const SEND_CHECKSUM_ERROR      = 1 << 0;
        const RECEIVE_CHECKSUM_ERROR   = 1 << 1;
        const SEND_ACCEPT_ERROR        = 1 << 2;
        const RECEIVE_ACCEPT_ERROR     = 1 << 3;
        const REDIRECTABLE_IPI         = 1 << 4;
        const SEND_ILLEGAL_VECTOR      = 1 << 5;
        const RECEIVE_ILLEGAL_VECTOR   = 1 << 6;
        const ILLEGAL_REGISTER_ADDRESS = 1 << 7;
    }
}

/// APIC Error interrupt event handler.
///
/// This ISR can be triggered on the following conditions:
/// * Sending an IPI to a CPU core that doesn't exist.
/// * Trying to trigger an illegal interrupt vector (like 0 to 15).
/// * A checksum error caused by electrical noise on the APIC bus.
///
/// These errors are highly recoverable and should not trigger a system crash.
///
/// It's possible to know what went wrong by checking the ESR (Error Status Register).
pub extern "x86-interrupt" fn handler(stack_frame: InterruptStackFrame) {
    // Fetch the CPU-local data.
    let cpu_data = CpuLocalData::current();

    // Get the base address of the LAPIC.
    let lapic_virt_addr = get_lapic_address();

    // The ESR offset is 0x280 bytes (that are 0xA0 in u32 increments).
    let esr_ptr: *mut u32 = lapic_virt_addr.add(LAPIC_ESR_REGISTER).as_mut_ptr();

    // Get the error flags present in the ESR.
    let (error_value, error_flags) = unsafe {
        // Write 0 to the ESR to force it to update its internal state.
        esr_ptr.write_volatile(0);

        // Read the actual error flags.
        let error_value = esr_ptr.read_volatile();
        (error_value, ApicErrorFlags::from_bits_truncate(error_value))
    };

    // Log the incident.
    debug!(
        r#"
            ### APIC ERROR ###
                ESR Value: {:#010X}
                Error Flags: {:?}
                Instruction Pointer: {:#X}
                Stack Frame:
                    {:#?}
            "#,
        error_value,
        error_flags,
        stack_frame.instruction_pointer.as_u64(),
        stack_frame
    );

    // Signal the LAPIC to proceed with the next event.
    without_interrupts(|| {
        let mut lapic = cpu_data.lapic();
        unsafe { lapic.end_of_interrupt() }
    });
}
