use crate::drivers::virtio::hal::QuayVirtIoHal;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use log::{error, trace};
use spin::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::pci::PciTransport;

lazy_static! {
    /// Global registry of all VirtIO block devices.
    /// Index 0 corresponds to /dev/vda, Index 1 to /dev/vdb, etc.
    pub static ref BLOCK_DEVICES: Mutex<Vec<VirtIOBlk<QuayVirtIoHal, PciTransport>>> =
        Mutex::new(Vec::new());
}

/// Initialize a block device which is bound to a PCI Transport.
pub fn init_block_device(transport: PciTransport) {
    trace!("Initializing VirtIO Block Device...");

    // Create the Block Device.
    let mut blk = match VirtIOBlk::<QuayVirtIoHal, _>::new(transport) {
        Ok(device) => device,
        Err(error) => {
            error!("Failed to initialize VirtIO Block Device: {error:?}");
            return;
        }
    };

    let capacity = blk.capacity();
    trace!(
        "VirtIO Block Device successfully initialized! Capacity: {} sectors ({} MiB)",
        capacity,
        (capacity * 512) / 1024 / 1024
    );

    // Quick read test for sanity checking.
    let mut buffer = [0u8; 512];
    match blk.read_blocks(0, &mut buffer) {
        Ok(_) => {
            trace!("Successfully read from VirtIO Block Device!");
            if buffer[510] == 0x55 && buffer[511] == 0xAA {
                trace!("Sector 0 contains a valid boot signature (0x55AA)!");
            }
        }
        Err(error) => error!("Failed to read from VirtIO Block Device: {error:?}"),
    }

    // Add the device to the global registry.
    BLOCK_DEVICES.lock().push(blk);
}

/// Helper function for the VFS to read blocks from a specific device.
/// `device_id`: 0 for vda, 1 for vdb, etc.
pub fn read_block(
    device_id: usize,
    sector: usize,
    buffer: &mut [u8],
) -> Result<(), virtio_drivers::Error> {
    let mut devices = BLOCK_DEVICES.lock();
    if device_id >= devices.len() {
        return Err(virtio_drivers::Error::InvalidParam);
    }
    devices[device_id].read_blocks(sector, buffer)
}

/// Helper function for the VFS to write blocks to a specific device.
pub fn write_block(
    device_id: usize,
    sector: usize,
    buffer: &[u8],
) -> Result<(), virtio_drivers::Error> {
    let mut devices = BLOCK_DEVICES.lock();
    if device_id >= devices.len() {
        return Err(virtio_drivers::Error::InvalidParam);
    }
    devices[device_id].write_blocks(sector, buffer)
}
