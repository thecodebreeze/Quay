use crate::drivers::pci::bar::PciBar;
use acpi::platform::PciConfigRegions;
use core::ops::{BitAnd, BitOr, Shl, Shr};
use core::ptr;

/// Represents a discovered PCIe Device.
#[derive(Debug)]
pub struct PciDevice {
    segment_group: u16,
    bus: u8,
    device: u8,
    function: u8,
    vendor_id: u16,
    device_id: u16,
}

impl PciDevice {
    pub fn new(
        segment_group: u16,
        bus: u8,
        device: u8,
        function: u8,
        vendor_id: u16,
        device_id: u16,
    ) -> Self {
        Self {
            segment_group,
            bus,
            device,
            function,
            vendor_id,
            device_id,
        }
    }

    pub fn read_bar(
        &self,
        regions: &PciConfigRegions,
        hhdm_offset: u64,
        bar_index: u8,
    ) -> Option<PciBar> {
        assert!(bar_index < 6, "PCIe devices only have 6 BARs (0-5).");

        let phys_addr =
            regions.physical_address(self.segment_group, self.bus, self.device, self.function)?;
        let virt_addr = phys_addr.saturating_add(hhdm_offset);

        // BARs start at offset 0x10. Each is 4 bytes.
        let bar_offset = bar_index.saturating_mul(4).saturating_add(0x10);
        let bar_value = unsafe {
            ptr::read_volatile(virt_addr.saturating_add(bar_offset as u64) as *const u32)
        };

        // If the BAR is 0, it is unconfigured/unused.
        if bar_value == 0 {
            return None;
        }

        // Bit 0 determines if this is a Memory space or I/O space BAR.
        let is_io = (bar_value & 0x1) != 0;
        if is_io {
            // I/O Bar - Bits 2-31 are the port address.
            return Some(PciBar::LegacyIo {
                port: bar_value.bitand(!0x03),
            });
        }

        // Memory BAR - Bits 1 - 2 determine the type (32-bit or 64-bit).
        let bar_type = bar_value.shr(1u32).bitand(0x03);

        // Bit 3 determines if the memory is prefetchable (safe for the CPU to aggressively cache).
        let prefetchable = bar_value.bitand(0x08).ne(&0);

        // Actual base address starts at bit 4.
        let base_address_mask = !0xF;
        if bar_type.eq(&0x00) {
            // 32-bit Memory BAR
            Some(PciBar::Memory32 {
                address: bar_value.bitand(base_address_mask),
                prefetchable,
            })
        } else if bar_type.eq(&0x02) {
            // 64-bit Memory BAR. They take two slots, the upper 32 bits are in the next BAR.
            assert!(bar_index.lt(&5), "64-bit BAR cannot be at index 5");
            let bar_high = unsafe {
                ptr::read_volatile(
                    virt_addr
                        .saturating_add(bar_offset as u64)
                        .saturating_add(4) as *const u32,
                )
            };
            let address = (bar_high as u64)
                .shl(32u64)
                .bitor(bar_value.bitand(base_address_mask) as u64);
            Some(PciBar::Memory64 {
                address,
                prefetchable,
            })
        } else {
            None
        }
    }

    pub fn segment_group(&self) -> u16 {
        self.segment_group
    }

    pub fn bus(&self) -> u8 {
        self.bus
    }

    pub fn device(&self) -> u8 {
        self.device
    }

    pub fn function(&self) -> u8 {
        self.function
    }

    pub fn vendor_id(&self) -> u16 {
        self.vendor_id
    }

    pub fn device_id(&self) -> u16 {
        self.device_id
    }
}
