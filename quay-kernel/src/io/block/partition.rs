use crate::io::block::{read_block, write_block, BLOCK_DEVICES};
use alloc::string::String;
use gpt_disk_io::gpt_disk_types::GptPartitionType;
use uguid::Guid;
use virtio_drivers::Error;

/// Represents a single partition on a block device.
#[derive(Debug, Clone)]
pub struct Partition {
    pub start_lba: u64,
    pub end_lba: u64,
    pub size_sectors: u64,
    pub name: String,
    pub guid: Guid,
    pub type_guid: GptPartitionType,
}

/// Reads blocks from a specific partition.
/// `relative_sector` is the offset from the START of the partition, not the disk!
pub fn read_partition(
    device_id: usize,
    partition_index: usize,
    relative_sector: usize,
    buffer: &mut [u8],
) -> Result<(), Error> {
    let absolute_sector = get_absolute_sector(device_id, partition_index, relative_sector, buffer)?;
    read_block(device_id, absolute_sector, buffer)
}

/// Writes blocks to a specific partition.
pub fn write_partition(
    device_id: usize,
    partition_index: usize,
    relative_sector: usize,
    buffer: &[u8],
) -> Result<(), Error> {
    let absolute_sector = get_absolute_sector(device_id, partition_index, relative_sector, buffer)?;
    write_block(device_id, absolute_sector, buffer)
}

fn get_absolute_sector(
    device_id: usize,
    partition_index: usize,
    relative_sector: usize,
    buffer: &[u8],
) -> Result<usize, Error> {
    let devices = BLOCK_DEVICES.lock();
    let device = devices.get(device_id).ok_or(Error::InvalidParam)?;
    let partition = device
        .partitions
        .get(partition_index)
        .ok_or(Error::InvalidParam)?;

    let num_sectors = (buffer.len() / 512) as u64;

    if (relative_sector as u64) + num_sectors > partition.size_sectors {
        return Err(Error::InvalidParam);
    }

    Ok((partition.start_lba + relative_sector as u64) as usize)
}
