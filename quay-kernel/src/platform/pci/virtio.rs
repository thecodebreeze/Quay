use crate::drivers::virtio::hal::QuayVirtIoHal;
use log::error;
use virtio_drivers::transport::pci::PciTransport;
use virtio_drivers::transport::pci::bus::{Cam, Command, DeviceFunction, MmioCam, PciRoot};

/// The Vendor ID for the VirtIO PCI Devices.
pub const VIRTIO_VENDOR: u16 = 0x1AF4;

/// Creates a VirtIO PCI Transport layer for a discovered device using the ECAM.
pub fn create_pci_transport(
    mcfg_base_virt_addr: u64,
    bus: u8,
    device: u8,
    function: u8,
) -> Option<PciTransport> {
    // Since we mapped the full 256MiB ECAM memory space, we can use Cam::Ecam here.
    let cam = unsafe { MmioCam::new(mcfg_base_virt_addr as *mut u8, Cam::Ecam) };
    let mut root = PciRoot::new(cam);

    // Create the VirtIO device function.
    let device_function = DeviceFunction {
        bus,
        device,
        function,
    };

    // Enable Bus Master and Memory Space for the DMA.
    let (_, mut command) = root.get_status_command(device_function);
    command |= Command::BUS_MASTER;
    command |= Command::MEMORY_SPACE;
    root.set_command(device_function, command);

    // Attempt to initialize the PCI transport using our HAL.
    match PciTransport::new::<QuayVirtIoHal, _>(&mut root, device_function) {
        Ok(transport) => Some(transport),
        Err(error) => {
            error!("Failed to create VirtIO PCI Transport: {error:?}");
            None
        }
    }
}
