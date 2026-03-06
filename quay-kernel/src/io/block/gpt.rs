use crate::io::block::partition::Partition;
use crate::io::block::{add_partition, read_block, write_block, BLOCK_DEVICES};
use alloc::format;
use gpt_disk_io::gpt_disk_types::{BlockSize, Lba};
use gpt_disk_io::{BlockIo, Disk};
use log::{error, trace};

/// A wrapper struct that allows `gpt-disk-rs` to talk to our block registry.
pub struct QuayBlockIo {
    device_id: usize,
}

impl QuayBlockIo {
    pub fn new(device_id: usize) -> Self {
        Self { device_id }
    }
}

impl BlockIo for QuayBlockIo {
    type Error = virtio_drivers::Error;

    fn block_size(&self) -> BlockSize {
        BlockSize::BS_512
    }

    fn num_blocks(&mut self) -> Result<u64, Self::Error> {
        let devices = BLOCK_DEVICES.lock();
        Ok(devices[self.device_id].driver.capacity())
    }

    fn read_blocks(&mut self, start_lba: Lba, dst: &mut [u8]) -> Result<(), Self::Error> {
        read_block(self.device_id, start_lba.to_u64() as usize, dst)
    }

    fn write_blocks(&mut self, start_lba: Lba, src: &[u8]) -> Result<(), Self::Error> {
        write_block(self.device_id, start_lba.to_u64() as usize, src)
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// Scans the specified device for GPT partitions and logs them.
pub fn scan_partitions(device_id: usize) {
    let io = QuayBlockIo::new(device_id);
    let mut disk = match Disk::new(io) {
        Ok(disk) => disk,
        Err(e) => {
            error!("Failed to initialize GPT Disk parser: {}", e);
            return;
        }
    };

    let mut block_buf = [0u8; 512];
    let header = match disk.read_primary_gpt_header(&mut block_buf) {
        Ok(header) => header,
        Err(e) => {
            error!("Failed to read GPT Header: {}", e);
            return;
        }
    };

    trace!("GPT Header found! Parsing partitions...");
    let layout = header.get_partition_entry_array_layout().unwrap();
    let iter = disk
        .gpt_partition_entry_array_iter(layout, &mut block_buf)
        .unwrap();

    for (partition_number, entry_result) in (1..).zip(iter) {
        let entry = entry_result.unwrap();
        if entry.is_used() {
            let start = entry.starting_lba.to_u64();
            let end = entry.ending_lba.to_u64();
            let size_sectors = end - start + 1;
            let size_mb = (size_sectors * 512) / 1024 / 1024;

            // Format the UTF-16 name into a standard Rust String
            let name = format!("{}", entry.name);

            trace!(
                "Partition {}: '{}' | Start LBA: {} | End LBA: {} | Size: {} MiB",
                partition_number, name, start, end, size_mb
            );

            // Store the partition in our global registry!
            add_partition(
                device_id,
                Partition {
                    start_lba: start,
                    end_lba: end,
                    size_sectors,
                    name,
                    guid: entry.unique_partition_guid,
                    type_guid: entry.partition_type_guid,
                },
            );
        }
    }
}
