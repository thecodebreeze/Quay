pub mod gpt;
pub mod partition;

use crate::drivers::virtio::hal::QuayVirtIoHal;
use crate::io::block::partition::Partition;
use alloc::vec::Vec;
use lazy_static::lazy_static;
use spin::Mutex;
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::pci::PciTransport;

lazy_static! {
    /// Global registry of all initialized block devices.
    pub static ref BLOCK_DEVICES: Mutex<Vec<BlockDevice>> = Mutex::new(Vec::new());
}

/// Represents a logical block device, including its hardware driver and partitions.
pub struct BlockDevice {
    pub driver: VirtIOBlk<QuayVirtIoHal, PciTransport>,
    pub partitions: Vec<Partition>,
}

/// Registers a new block device and returns its assigned ID (0 for vda, 1 for vdb, etc.)
pub fn register_device(device: VirtIOBlk<QuayVirtIoHal, PciTransport>) -> usize {
    let mut devices = BLOCK_DEVICES.lock();
    let id = devices.len();
    devices.push(BlockDevice {
        driver: device,
        partitions: Vec::new(),
    });
    id
}

/// Adds a discovered partition to a registered block device.
pub fn add_partition(device_id: usize, partition: Partition) {
    let mut devices = BLOCK_DEVICES.lock();
    if let Some(device) = devices.get_mut(device_id) {
        device.partitions.push(partition);
    }
}

/// Helper function for the VFS/GPT to read blocks from a specific device.
pub fn read_block(
    device_id: usize,
    sector: usize,
    buffer: &mut [u8],
) -> Result<(), virtio_drivers::Error> {
    let mut devices = BLOCK_DEVICES.lock();
    if device_id >= devices.len() {
        return Err(virtio_drivers::Error::InvalidParam);
    }
    devices[device_id].driver.read_blocks(sector, buffer)
}

/// Helper function for the VFS/GPT to write blocks to a specific device.
pub fn write_block(
    device_id: usize,
    sector: usize,
    buffer: &[u8],
) -> Result<(), virtio_drivers::Error> {
    let mut devices = BLOCK_DEVICES.lock();
    if device_id >= devices.len() {
        return Err(virtio_drivers::Error::InvalidParam);
    }
    devices[device_id].driver.write_blocks(sector, buffer)
}
