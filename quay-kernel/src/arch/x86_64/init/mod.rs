use alloc::boxed::Box;
use log::info;

/// Initializes early CPU structures like the GDT and IDT.
pub fn initialize_cpu() {
    info!("Loading the GDT and IDT...");
    super::gdt::load_global_descriptor_table();
    super::idt::load_interrupt_descriptor_table();
    info!("GDT and IDT loaded!");
}

/// Initializes platform-specific hardware using ACPI tables.
pub fn initialize_hardware(rsdp_phys_addr: u64, rsdp_virt_addr: u64, hhdm_offset: u64) {
    use crate::sys::acpi::QuayAcpiHandler;

    // Parse the ACPI tables.
    let acpi_handler = QuayAcpiHandler::new(hhdm_offset);
    let lapic_phys_addr = acpi_handler.get_lapic_phys_addr(rsdp_virt_addr);

    // Initialize the TSC and register it as the global system timer.
    info!("Initializing the TSC...");
    let tsc = crate::drivers::timer::tsc::Tsc::initialize();
    crate::sys::time::register_timer(Box::new(tsc));
    info!("TSC Initialized!");

    // Configure the APIC.
    info!("Configuring the APIC...");
    super::pic::disable_legacy_pic();

    let (lapic_id, lapic) = super::apic::initialize_local_apic(lapic_phys_addr, hhdm_offset);
    info!("APIC configured! LAPIC ID: {}", lapic_id);

    // 4. Load the CPU Local Data
    info!("Loading CPU data...");
    super::cpu::CpuLocalData::load(lapic_id, lapic);
    info!("CPU data loaded!");
}

#[inline(always)]
pub fn enable_interrupts() {
    x86_64::instructions::interrupts::enable();
}

#[inline(always)]
pub fn halt() {
    x86_64::instructions::hlt();
}
