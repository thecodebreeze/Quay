//! This module implements APIC related utilities.
use crate::arch::x86_64::cpu;
use crate::arch::x86_64::idt::{
    APIC_ERROR_VECTOR_INDEX, APIC_SPURIOUS_VECTOR_INDEX, APIC_TIMER_VECTOR_INDEX,
};
use crate::sys::memory::vmm::{get_vmm, map_mmio_range};
use core::sync::atomic;
use core::sync::atomic::AtomicU64;
use log::{error, info, trace};
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};
use x86_64::registers::model_specific::Msr;
use x86_64::VirtAddr;

/// Atomic variable to store the virtual address of the LAPIC.
static LAPIC_VADDR: AtomicU64 = AtomicU64::new(0);

const IA32_TSC_DEADLINE: u32 = 0x6E0;

/// Offset of the Error Status Register.
pub const LAPIC_ESR_REGISTER: u64 = 0x280;

/// Initialize the LAPIC of the current CPU.
///
/// This must be called ONCE PER CPU!
pub fn initialize_local_apic(lapic_phys_addr: u64, hhdm_offset: u64) -> (u32, LocalApic) {
    assert!(
        cpu::feature::has_tsc_deadline(),
        "TSC-Deadline mode not supported by this CPU!"
    );
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
        .timer_mode(TimerMode::TscDeadline)
        .build()
        .expect("Failed to initialize Local APIC");

    unsafe { lapic.enable() };

    // Enable the LAPIC and return the ID and the LAPIC itself.
    (unsafe { lapic.id() }, lapic)
}

/// Schedules a LAPIC interrupt to fire at an absolute TSC cycle.
pub fn schedule_event_absolute(target_tsc: u64) {
    let mut msr = Msr::new(IA32_TSC_DEADLINE);
    unsafe {
        msr.write(target_tsc);
    }
}

/// Schedules a LAPIC interrupt to fire in a specific number of cycles from now.
pub fn schedule_event_relative(cycles_from_now: u64) {
    let current_tsc = unsafe { core::arch::x86_64::_rdtsc() };
    schedule_event_absolute(current_tsc + cycles_from_now);
}

/// Get the current virtual address of the LAPIC.
#[inline]
pub fn get_lapic_address() -> VirtAddr {
    let virt_addr = LAPIC_VADDR.load(atomic::Ordering::Relaxed);
    assert_ne!(virt_addr, 0, "LAPIC Virtual Address not set yet");
    VirtAddr::new(virt_addr)
}
