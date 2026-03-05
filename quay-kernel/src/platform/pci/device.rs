#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciDeviceClass {
    VirtioBlock,
    VirtioNet,
    VirtioGpu,
    VirtioInput,
    VirtioRng,
    VirtioSocket,
    VirtioConsole,
    UsbController,
    Other(u8, u8),
}

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    bus: u8,
    device: u8,
    function: u8,
    vendor_id: u16,
    device_id: u16,
    class: PciDeviceClass,
    config_base_phys_address: u64,
}

impl PciDevice {
    pub fn new(
        bus: u8,
        device: u8,
        function: u8,
        vendor_id: u16,
        device_id: u16,
        class: PciDeviceClass,
        config_base_phys_address: u64,
    ) -> Self {
        Self {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class,
            config_base_phys_address,
        }
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

    pub fn class(&self) -> PciDeviceClass {
        self.class
    }

    pub fn config_base_phys_address(&self) -> u64 {
        self.config_base_phys_address
    }
}
