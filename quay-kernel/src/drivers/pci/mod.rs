use crate::sys::memory::vmm::{get_vmm, map_mmio_range};
use acpi::platform::PciConfigRegions;
use alloc::vec::Vec;
use core::ptr;
use core::ptr::read_volatile;
use log::{debug, warn};

pub mod bar;
pub mod device;

/// Scans the PCIe bus using ECAM and returns a list of discovered devices.
pub fn enumerate_pci_devices(
    regions: &PciConfigRegions,
    hhdm_offset: u64,
) -> Vec<device::PciDevice> {
    let mut discovered_devices = Vec::new();

    // It's fine to lock the VMM here since we don't want our allocations to be messed up.
    let mut vmm = get_vmm();

    // Iterate over every ECAM region (some massive server motherboards have multiple regions).
    for region in regions.regions.iter() {
        let region_base_addr = region.base_address;
        let segment_group = region.pci_segment_group;

        // Calculate the exact size of this ECAM region.
        // Each bus takes exactly 1 MiB (32 devices * 8 functions * 4096 bytes).
        let num_buses = (region.bus_number_end as u64)
            .saturating_add(1)
            .saturating_sub(region.bus_number_start as u64);
        let region_size_bytes = num_buses.saturating_mul(1024).saturating_mul(1024);

        // Map the MMIO region into our page tables.
        debug!(
            "Mapping PCIe ECAM region at {:#X} ({} MiB)...",
            region_base_addr,
            region_size_bytes / (1024 * 1024)
        );
        if let Err(error) = map_mmio_range(
            &mut *vmm,
            region.base_address,
            region_size_bytes,
            hhdm_offset,
        ) {
            panic!("Failed to map PCIe ECAM region: {:?}", error);
        }

        // Iterate through all Buses managed by this specific segment.
        for bus in region.bus_number_start..=region.bus_number_end {
            // The PCIe spec determines 32 devices per bus...
            for device in 0..32 {
                // ... and 8 Functions per Device.
                for function in 0..8 {
                    // Let the acpi crate calculate the exact physical address for this BDF.
                    let Some(phys_addr) =
                        regions.physical_address(segment_group, bus, device, function)
                    else {
                        warn!("Invalid BDF address. Skipping.");
                        continue;
                    };

                    // Since our HHDM is 1-to-1 we can simply add it.
                    let virt_addr = phys_addr.saturating_add(hhdm_offset);

                    // Read the first 16 bits (Vendor ID).
                    let vendor_id = unsafe { ptr::read_volatile(virt_addr as *const u16) };

                    // 0xFFFF means there is no silicon plugged into this slot.
                    if vendor_id == 0xFFFF {
                        if function == 0 {
                            // If Function 0 doesn't exist, the whole device slot is empty.
                            break;
                        }
                        continue;
                    }

                    // Read the next 16 bits (Device ID) at offset 0x02.
                    let device_id =
                        unsafe { ptr::read_volatile(virt_addr.saturating_add(0x02) as *const u16) };

                    discovered_devices.push(device::PciDevice::new(
                        segment_group,
                        bus,
                        device,
                        function,
                        vendor_id,
                        device_id,
                    ));

                    // To check if a device is a Multi-Function Device (MFD), read the Header Type
                    // at offset 0x0E. If bit 7 is 0, it only has one function, so we can break this
                    // function loop earlier.
                    if function == 0 {
                        let header_type =
                            unsafe { read_volatile(virt_addr.saturating_add(0x0E) as *const u8) };
                        if (header_type & 0x80) == 0 {
                            break;
                        }
                    }
                }
            }
        }
    }

    discovered_devices
}
