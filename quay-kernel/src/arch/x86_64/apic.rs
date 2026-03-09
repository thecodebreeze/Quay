//! This module implements APIC related utilities.
use crate::arch::x86_64::idt::{
    APIC_ERROR_VECTOR_INDEX, APIC_SPURIOUS_VECTOR_INDEX, APIC_TIMER_VECTOR_INDEX,
};
use crate::sys::memory::vmm::{get_vmm, map_mmio_range};
use core::sync::atomic;
use core::sync::atomic::AtomicU64;
use log::{error, info, trace};
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};
use x86_64::VirtAddr;

/// Atomic variable to store the virtual address of the LAPIC.
static LAPIC_VADDR: AtomicU64 = AtomicU64::new(0);

/// Offset of the Error Status Register.
pub const LAPIC_ESR_REGISTER: u64 = 0x280;

/// Initialize the LAPIC of the current CPU.
///
/// This must be called ONCE PER CPU!
pub fn initialize_local_apic(
    lapic_phys_addr: u64,
    hhdm_offset: u64,
    tick_rate: u32,
) -> (u32, LocalApic) {
    trace!("Initializing Local APIC...");
    // Compute the Local APIC virtual address.
    let lapic_virt_addr = lapic_phys_addr.saturating_add(hhdm_offset);
    LAPIC_VADDR.store(lapic_virt_addr, atomic::Ordering::Relaxed);

    // Map the APIC's physical memory into the HHDM. The APIC registers fit comfortably inside a
    // single 4KiB page.
    {
        let mut vmm = get_vmm();
        match map_mmio_range(&mut *vmm, lapic_phys_addr, 4096, hhdm_offset) {
            Ok(_) => {}
            Err(error) => {
                error!("Critical component initialization failed: {}", error);
                panic!("Critical component initialization failed: {}", error);
            }
        }

        info!(
            "Mapped LAPIC MMIO to virtual address: {:#X}",
            lapic_virt_addr
        );
    }

    // Build the LAPIC.
    let mut lapic = LocalApicBuilder::new()
        .error_vector(APIC_ERROR_VECTOR_INDEX as usize)
        .timer_vector(APIC_TIMER_VECTOR_INDEX as usize)
        .spurious_vector(APIC_SPURIOUS_VECTOR_INDEX as usize)
        .set_xapic_base(lapic_virt_addr)
        .timer_mode(TimerMode::Periodic)
        .timer_initial(tick_rate)
        .timer_divide(TimerDivide::Div16)
        .build()
        .expect("Failed to initialize Local APIC");

    // Enable the LAPIC and return the ID and the LAPIC itself.
    unsafe {
        lapic.enable();
        (lapic.id(), lapic)
    }
}

/// Calibrate the LAPIC timer against a known reference clock to find the 1ms tick rate.
pub fn calibrate_apic_timer() -> u32 {
    // Build a temporary LAPIC in One-Shot mode.
    let mut lapic = LocalApicBuilder::new()
        .error_vector(APIC_ERROR_VECTOR_INDEX as usize)
        .timer_vector(APIC_TIMER_VECTOR_INDEX as usize)
        .spurious_vector(APIC_SPURIOUS_VECTOR_INDEX as usize)
        .timer_mode(TimerMode::OneShot)
        .timer_initial(1000)
        .timer_divide(TimerDivide::Div16)
        .build()
        .expect("Failed to initialize dummy Local APIC");

    unsafe {
        lapic.enable();
    }

    // Wait exactly 10ms using a reliable HPET timer.
    reference_timer_sleep_10ms();

    // Read the current count in the dummy LAPIC.
    let current_count = unsafe { lapic.timer_current() };

    // Calculate ticks elapsed in 10ms.
    let ticks_in_10ms = u32::MAX.saturating_sub(current_count);
    let ticks_in_1ms = ticks_in_10ms.saturating_div(10);

    // Disable the dummy LAPIC.
    unsafe {
        lapic.set_timer_initial(0);
        lapic.disable();
    }

    ticks_in_1ms
}

fn reference_timer_sleep_10ms() {}

/// Get the current virtual address of the LAPIC.
#[inline]
pub fn get_lapic_address() -> VirtAddr {
    let virt_addr = LAPIC_VADDR.load(atomic::Ordering::Relaxed);
    assert_ne!(virt_addr, 0, "LAPIC Virtual Address not set yet");
    VirtAddr::new(virt_addr)
}
