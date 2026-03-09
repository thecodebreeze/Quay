use crate::arch::x86_64::cpu::CpuLocalData;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::idt::InterruptStackFrame;

/// Handles the APIC Timer event.
pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    // Fetch the CPU-local data.
    let cpu_data = CpuLocalData::current();

    // Signal the LAPIC to proceed with the next event.
    without_interrupts(|| {
        let mut lapic = cpu_data.lapic();
        unsafe { lapic.end_of_interrupt() }
    });
}
