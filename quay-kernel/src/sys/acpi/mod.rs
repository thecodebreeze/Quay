use crate::{arch, sys};
use acpi::aml::AmlError;
use acpi::platform::PciConfigRegions;
use acpi::{AcpiTables, Handle, PciAddress, PhysicalMapping};
use core::ptr;
use log::{error, trace};

/// ACPI Handler is used to interact with the ACPI standard.
///
/// The handler implementation teaches the [acpi] crate how to access physical memory; since Limine
/// maps all physical memory at `HHDM_OFFSET`, we just add it!
#[derive(Clone, Copy)]
pub struct QuayAcpiHandler {
    hhdm_offset: u64,
}

impl QuayAcpiHandler {
    pub fn new(hhdm_offset: u64) -> Self {
        Self { hhdm_offset }
    }

    /// Fetches the Local APIC physical address in memory from the MDAT.
    #[cfg(target_arch = "x86_64")]
    pub fn get_lapic_phys_addr(&self, rsdp_address: u64) -> u64 {
        // Convert the virtual address to a physical address.
        let phys_addr = rsdp_address.saturating_sub(self.hhdm_offset);

        // Fetch the RSDT/XSDT.
        trace!("Fetching the ACPI Tables ({:#X})...", phys_addr);
        let tables = unsafe {
            AcpiTables::from_rsdp(*self, phys_addr as usize).expect("ACPI tables to be present")
        };

        // Fetch the MADT.
        trace!("Fetching the MADT...");
        let madt = tables
            .find_table::<acpi::sdt::madt::Madt>()
            .expect("MADT to be present");

        madt.get().local_apic_address as u64
    }

    /// Fetches the PCIe ECAM regions.
    ///
    /// This works on all architectures.
    pub fn get_pci_regions(&self, rsdp_address: u64) -> PciConfigRegions {
        let phys_addr = rsdp_address.saturating_sub(self.hhdm_offset);

        let tables = unsafe {
            match AcpiTables::from_rsdp(*self, phys_addr as usize) {
                Ok(tables) => tables,
                Err(error) => {
                    error!("Failed to fetch ACPI tables: {:?}", error);
                    panic!("Failed to fetch ACPI tables");
                }
            }
        };

        trace!("Fetching the MCFG (PCIe) Table...");

        match PciConfigRegions::new(&tables) {
            Ok(regions) => regions,
            Err(error) => {
                error!("Failed to fetch PCIe MCFG: {:?}", error);
                panic!("Failed to fetch PCIe MCFG");
            }
        }
    }
}

impl acpi::Handler for QuayAcpiHandler {
    unsafe fn map_physical_region<T>(
        &self,
        physical_address: usize,
        size: usize,
    ) -> PhysicalMapping<Self, T> {
        let virt_addr = physical_address as u64 + self.hhdm_offset;
        PhysicalMapping {
            physical_start: physical_address,
            virtual_start: ptr::NonNull::new(virt_addr as *mut T).unwrap(),
            region_length: size,
            mapped_length: size,
            handler: *self,
        }
    }

    fn unmap_physical_region<T>(_region: &PhysicalMapping<Self, T>) {
        // Memory is permanently mapped. Nothing to unmap.
    }

    fn read_u8(&self, address: usize) -> u8 {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::read_volatile(virt_addr as *const u8) }
    }

    fn read_u16(&self, address: usize) -> u16 {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::read_volatile(virt_addr as *const u16) }
    }

    fn read_u32(&self, address: usize) -> u32 {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::read_volatile(virt_addr as *const u32) }
    }

    fn read_u64(&self, address: usize) -> u64 {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::read_volatile(virt_addr as *const u64) }
    }

    fn write_u8(&self, address: usize, value: u8) {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::write_volatile(virt_addr as *mut u8, value) }
    }

    fn write_u16(&self, address: usize, value: u16) {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::write_volatile(virt_addr as *mut u16, value) }
    }

    fn write_u32(&self, address: usize, value: u32) {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::write_volatile(virt_addr as *mut u32, value) }
    }

    fn write_u64(&self, address: usize, value: u64) {
        let virt_addr = address.saturating_add(self.hhdm_offset as usize);
        unsafe { ptr::write_volatile(virt_addr as *mut u64, value) }
    }

    fn read_io_u8(&self, port: u16) -> u8 {
        arch::target::port_io::read_u8(port)
    }

    fn read_io_u16(&self, port: u16) -> u16 {
        arch::target::port_io::read_u16(port)
    }

    fn read_io_u32(&self, port: u16) -> u32 {
        arch::target::port_io::read_u32(port)
    }

    fn write_io_u8(&self, port: u16, value: u8) {
        arch::target::port_io::write_u8(port, value);
    }

    fn write_io_u16(&self, port: u16, value: u16) {
        arch::target::port_io::write_u16(port, value);
    }

    fn write_io_u32(&self, port: u16, value: u32) {
        arch::target::port_io::write_u32(port, value);
    }

    fn read_pci_u8(&self, _address: PciAddress, _offset: u16) -> u8 {
        unimplemented!()
    }

    fn read_pci_u16(&self, _address: PciAddress, _offset: u16) -> u16 {
        unimplemented!()
    }

    fn read_pci_u32(&self, _address: PciAddress, _offset: u16) -> u32 {
        unimplemented!()
    }

    fn write_pci_u8(&self, _address: PciAddress, _offset: u16, _value: u8) {
        unimplemented!()
    }

    fn write_pci_u16(&self, _address: PciAddress, _offset: u16, _value: u16) {
        unimplemented!()
    }

    fn write_pci_u32(&self, _address: PciAddress, _offset: u16, _value: u32) {
        unimplemented!()
    }

    fn nanos_since_boot(&self) -> u64 {
        sys::time::nanos_since_boot()
    }

    fn stall(&self, microseconds: u64) {
        sys::time::stall_us(microseconds);
    }

    fn sleep(&self, milliseconds: u64) {
        sys::time::sleep_ms(milliseconds);
    }

    fn create_mutex(&self) -> Handle {
        unimplemented!()
    }

    fn acquire(&self, _mutex: Handle, _timeout: u16) -> Result<(), AmlError> {
        unimplemented!()
    }

    fn release(&self, _mutex: Handle) {
        unimplemented!()
    }
}
