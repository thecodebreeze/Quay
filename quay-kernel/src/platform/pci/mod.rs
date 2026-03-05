mod device;
mod virtio;

pub use device::*;

use crate::platform::pci::virtio::VIRTIO_VENDOR;
use acpi::platform::PciConfigRegions;
use alloc::vec;
use alloc::vec::Vec;
use core::ptr::read_volatile;
use log::{info, trace};

pub fn scan_pci_devices(pci_regions: &PciConfigRegions, hhdm_offset: u64) -> Vec<PciDevice> {
    let mut devices = Vec::new();
    trace!("Scanning PCI bus via ECAM...",);

    let segment_group = 0;
    for bus in 0..=255 {
        for device in 0..32 {
            // Ask ACPI for the physical address of Function 0
            let Some(function_phys_address) =
                pci_regions.physical_address(segment_group, bus, device, 0)
            else {
                // No MCFG region covers this bus.
                continue;
            };

            // Read the Vendor ID. If it's 0xFFFF, there is no device in this slot.
            let vendor_id = read_pci_u16(function_phys_address, hhdm_offset, 0x00);
            if vendor_id == 0xFFFF {
                continue;
            }

            for function in 0..8 {
                let Some(function_phys_address) =
                    pci_regions.physical_address(segment_group, bus, device, function)
                else {
                    continue;
                };

                let vendor_id = read_pci_u16(function_phys_address, hhdm_offset, 0x00);
                if vendor_id == 0xFFFF {
                    continue;
                }

                let device_id = read_pci_u16(function_phys_address, hhdm_offset, 0x02);

                // Offset 0x0A contains the Class (high byte) and Subclass (low byte)
                let class_info = read_pci_u16(function_phys_address, hhdm_offset, 0x0A);
                let class_code = (class_info >> 8) as u8;
                let subclass = (class_info & 0xFF) as u8;

                let device_class = categorize_device(vendor_id, device_id, class_code, subclass);

                let pci_dev = PciDevice::new(
                    bus,
                    device,
                    function,
                    vendor_id,
                    device_id,
                    device_class,
                    function_phys_address,
                );

                trace!(
                    "Found PCI Device - Bus: {:02X}, Dev: {:02X}, Func: {} | Vendor: {:#06X}, Class: {:?}",
                    bus, device, function, vendor_id, device_class
                );

                devices.push(pci_dev);
            }
        }
    }

    devices
}

fn categorize_device(
    vendor_id: u16,
    device_id: u16,
    class_code: u8,
    subclass: u8,
) -> PciDeviceClass {
    if vendor_id == VIRTIO_VENDOR {
        match device_id {
            0x1000 | 0x1041 => PciDeviceClass::VirtioNet,
            0x1001 | 0x1042 => PciDeviceClass::VirtioBlock,
            0x1002 | 0x1043 => PciDeviceClass::VirtioConsole,
            0x1003 | 0x1044 => PciDeviceClass::VirtioRng,
            0x1050 => PciDeviceClass::VirtioGpu,
            0x1052 => PciDeviceClass::VirtioInput,
            0x1053 => PciDeviceClass::VirtioSocket,
            _ => PciDeviceClass::Other(class_code, subclass),
        }
    } else if class_code == 0x0C && subclass == 0x03 {
        PciDeviceClass::UsbController
    } else {
        PciDeviceClass::Other(class_code, subclass)
    }
}

/// Reads a 16-bit value from the PCI Express Configuration Space using MMIO.
fn read_pci_u16(config_base_phys: u64, hhdm: u64, offset: u16) -> u16 {
    let virt_addr = config_base_phys + hhdm + (offset as u64);

    unsafe { read_volatile(virt_addr as *const u16) }
}
