use acpi::aml::AmlError;
use acpi::platform::interrupt::Apic;
use acpi::{Handle, PciAddress, PhysicalMapping};
use core::ptr::{NonNull, read_volatile, write_volatile};
use x86_64::instructions::port::Port;

#[derive(Clone)]
pub struct QuayAcpiHandler {
    // We store the HHDM offset so we can convert physical addresses to virtual ones.
    hhdm_offset: u64,
}

/// Search the ACPI table for an IOAPIC device.
pub fn search_acpi_for_apic(
    handler: QuayAcpiHandler,
    rsdp_physical_address: usize,
) -> Option<Apic> {
    // Let the crate parse the entire ACPI tree!
    let acpi_tables = unsafe {
        acpi::AcpiTables::from_rsdp(handler.clone(), rsdp_physical_address)
            .expect("Failed to parse ACPI tables")
    };

    // Extract the Platform Info (Renamed in v6 to AcpiPlatform).
    //
    // It takes the tables object entirely instead of a reference.
    let platform_info = acpi::platform::AcpiPlatform::new(acpi_tables, handler)
        .expect("Failed to get Platform Info");

    if let acpi::platform::InterruptModel::Apic(apic_info) = platform_info.interrupt_model {
        log::trace!(
            "Local APIC physical address: {:#X}",
            apic_info.local_apic_address
        );

        for io_apic in apic_info.io_apics.iter() {
            log::trace!(
                "Found IOAPIC at physical address: {:#X} (Global System Interrupt Base: {})",
                io_apic.address,
                io_apic.global_system_interrupt_base
            );
        }

        Some(apic_info)
    } else {
        log::error!("System does not support APIC! We are stuck in the 80s.");
        None
    }
}

impl QuayAcpiHandler {
    pub fn new(hhdm_offset: u64) -> Self {
        Self { hhdm_offset }
    }
}

impl acpi::Handler for QuayAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        // Convert the physical address to a virtual HHDM address.
        let virtual_address = physical_address as u64 + self.hhdm_offset;

        // Version 6 uses a struct instead of a constructor method.
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: NonNull::new(virtual_address as *mut _).unwrap(),
            region_length: size,
            mapped_length: size,
            handler: self.clone(),
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // HHDM mapping is permanent, so we should never unmap it.
    }

    fn read_u8(&self, address: usize) -> u8 {
        unsafe { read_volatile(address as *const u8) }
    }

    fn read_u16(&self, address: usize) -> u16 {
        unsafe { read_volatile(address as *const u16) }
    }

    fn read_u32(&self, address: usize) -> u32 {
        unsafe { read_volatile(address as *const u32) }
    }

    fn read_u64(&self, address: usize) -> u64 {
        unsafe { read_volatile(address as *const u64) }
    }

    fn write_u8(&self, address: usize, value: u8) {
        unsafe { write_volatile(address as *mut u8, value) }
    }

    fn write_u16(&self, address: usize, value: u16) {
        unsafe { write_volatile(address as *mut u16, value) }
    }

    fn write_u32(&self, address: usize, value: u32) {
        unsafe { write_volatile(address as *mut u32, value) }
    }

    fn write_u64(&self, address: usize, value: u64) {
        unsafe { write_volatile(address as *mut u64, value) }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        unsafe { Port::new(port).read() }
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        unsafe { Port::new(port).read() }
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        unsafe { Port::new(port).write(value) }
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        unsafe { Port::new(port).write(value) }
    }

    fn read_pci_u8(&self, _address: PciAddress, _offset: u16) -> u8 {
        0
    }

    fn read_pci_u16(&self, _address: PciAddress, _offset: u16) -> u16 {
        0
    }

    fn read_pci_u32(&self, _address: PciAddress, _offset: u16) -> u32 {
        0
    }

    fn write_pci_u8(&self, _address: PciAddress, _offset: u16, _value: u8) {}

    fn write_pci_u16(&self, _address: PciAddress, _offset: u16, _value: u16) {}

    fn write_pci_u32(&self, _address: PciAddress, _offset: u16, _value: u32) {}

    fn nanos_since_boot(&self) -> u64 {
        0
    }

    fn stall(&self, _microseconds: u64) {}

    fn sleep(&self, _milliseconds: u64) {}

    fn create_mutex(&self) -> Handle {
        todo!()
    }

    fn acquire(&self, _mutex: Handle, _timeout: u16) -> Result<(), AmlError> {
        todo!()
    }

    fn release(&self, _mutex: Handle) {
        todo!()
    }
}
