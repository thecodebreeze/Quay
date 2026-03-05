use crate::x86::acpi::QuayAcpiHandler;
use acpi::AcpiTables;
use acpi::sdt::hpet::HpetTable;
use core::ptr::{read_volatile, write_volatile};
use x2apic::lapic::{LocalApic, TimerDivide, TimerMode};

// HPET Register Offsets
const HPET_CAPABILITIES: usize = 0x00;
const HPET_CONFIGURATION: usize = 0x10;
const HPET_MAIN_COUNTER: usize = 0xF0;

/// Find the HPET physical address from the ACPI tables.
pub fn get_hpet_base_addr(acpi_tables: &AcpiTables<QuayAcpiHandler>) -> usize {
    acpi_tables
        .find_table::<HpetTable>()
        .expect("HPET table not found! Ensure HPET is enabled in QEMU/BIOS.")
        .base_address
        .address as usize
}

/// Performs a calibration of the LAPIC timer to see how many LAPIC ticks occur in 1ms of
/// real-world time as measured by the HPET.
pub fn calibrate_apic_timer(lapic: &mut LocalApic, hpet_base_virt: *mut u64) -> u32 {
    unsafe {
        // Globally Enable the APIC (Software Enable)
        lapic.enable();
        lapic.set_timer_divide(TimerDivide::Div16);

        // Calculate HPET frequency dynamically
        let caps = read_volatile(hpet_base_virt);
        let fs_per_tick = (caps >> 32) as u64;

        // Safety check to prevent Division by Zero (which causes a fault)
        if fs_per_tick == 0 {
            log::error!("HPET reports 0 femtoseconds per tick. Check QEMU settings.");
            return 1_000_000; // Fallback
        }

        let target_hpet_ticks = 10_000_000_000_000 / fs_per_tick; // Ticks in 10ms

        // Enable HPET Main Counter
        let config_ptr = hpet_base_virt.add(HPET_CONFIGURATION / 8);
        write_volatile(config_ptr, read_volatile(config_ptr) | 1);

        // Run the measurement
        write_volatile(
            hpet_base_virt.add(HPET_MAIN_COUNTER / 8),
            HPET_CAPABILITIES as u64,
        );

        lapic.set_timer_initial(u32::MAX);
        lapic.set_timer_mode(TimerMode::OneShot);

        while read_volatile(hpet_base_virt.add(HPET_MAIN_COUNTER / 8)) < target_hpet_ticks {}

        let elapsed = u32::MAX - lapic.timer_current();
        elapsed / 10
    }
}
