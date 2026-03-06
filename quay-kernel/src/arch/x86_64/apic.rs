//! This module implements APIC related utilities.
use crate::arch::x86_64::cpu::CpuLocalData;
use crate::arch::x86_64::idt::{
    APIC_ERROR_VECTOR_INDEX, APIC_SPURIOUS_VECTOR_INDEX, APIC_TIMER_VECTOR_INDEX,
};
use crate::HHDM_REQUEST;
use core::sync::atomic;
use core::sync::atomic::AtomicU64;
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};
use x86_64::instructions::interrupts::without_interrupts;
use x86_64::registers::model_specific::Msr;
use x86_64::VirtAddr;

/// Atomic variable to store the virtual address of the LAPIC.
static LAPIC_VADDR: AtomicU64 = AtomicU64::new(0);

/// MSR index that holds the APIC Base Address.
const IA32_APIC_BASE_MSR: u32 = 0x1B;

/// Bits 12-51 hold the physical base.
const LAPIC_PHYS_ADDR_MASK: u64 = 0xFFFF_FFFF_FFFF_F000;

/// Offset of the End of Interrupt register.
const LAPIC_EOI_REGISTER: u64 = 0xB0;

/// Offset of the Spurious Interrupt Register.
const LAPIC_SPURIOUS_REGISTER: u64 = 0xF0;

/// Offset of the Local Vector Table Error Register.
const LAPIC_LVT_ERROR_REGISTER: u64 = 0x370;

/// Offset of the Local Vector Table Timer Register.
const LAPIC_LVT_TIMER_REGISTER: u64 = 0x320;

/// Offset of the Error Status Register.
pub const LAPIC_ESR_REGISTER: u64 = 0x280;

/// Loads the Global LAPIC Virtual Address.
///
/// This is run only once in the BSP (Bootstrap Processor/Core 0) during early boot.
pub fn load_global_lapic_address() {
    // Get the HHDM offset from Limine.
    let hhdm_offset = HHDM_REQUEST
        .get_response()
        .expect("HHDM response to be present")
        .offset();

    // Read the APIC Base MSR to get the true physical address.
    let apic_msr = Msr::new(IA32_APIC_BASE_MSR);
    let msr_value = unsafe { apic_msr.read() };

    // Mask out the flags (like the Enable bit and BSP bit) to get just the address.
    let phys_addr = msr_value & LAPIC_PHYS_ADDR_MASK;

    // Calculate the virtual address.
    let virt_addr = phys_addr + hhdm_offset;

    // Store it globally for all core to use.
    LAPIC_VADDR.store(virt_addr, atomic::Ordering::Relaxed);
}

/// Initialize the LAPIC of the current CPU.
///
/// This must be called ONCE PER CPU!
pub fn get_local_apic(tick_rate: u32) -> (u32, LocalApic) {
    // Load the base virtual address of the LAPIC.
    let lapic_base_virt_addr = get_lapic_address();

    // Build the LAPIC.
    let mut lapic = LocalApicBuilder::new()
        .error_vector(APIC_ERROR_VECTOR_INDEX as usize)
        .timer_vector(APIC_TIMER_VECTOR_INDEX as usize)
        .spurious_vector(APIC_SPURIOUS_VECTOR_INDEX as usize)
        .set_xapic_base(lapic_base_virt_addr.as_u64())
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

/// Get the current virtual address of the LAPIC.
#[inline]
pub fn get_lapic_address() -> VirtAddr {
    let virt_addr = LAPIC_VADDR.load(atomic::Ordering::Relaxed);
    assert_ne!(virt_addr, 0, "LAPIC Virtual Address not set yet");
    VirtAddr::new(virt_addr)
}
