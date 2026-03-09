use crate::arch::x86_64::cpu::CpuLocalData;
use crate::arch::x86_64::timer;
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::structures::idt::InterruptStackFrame;

/// We set Core 0 to always be the timekeeper.
const TIMEKEEPER_LAPIC_ID: u32 = 0;

/// Handles the APIC Timer event.
pub extern "x86-interrupt" fn handler(_stack_frame: InterruptStackFrame) {
    // Fetch the CPU-local data.
    let cpu_data = CpuLocalData::current();

    // If this is the timekeeper, then we need to update the system tick counter.
    if cpu_data.lapic_id().eq(&TIMEKEEPER_LAPIC_ID) {
        timer::increment_system_ticks();
    }

    // Signal the LAPIC to proceed with the next event.
    without_interrupts(|| {
        let mut lapic = cpu_data.lapic();
        unsafe { lapic.end_of_interrupt() }
    });
}
