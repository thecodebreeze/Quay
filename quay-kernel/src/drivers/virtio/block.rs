use crate::drivers::virtio::hal::QuayVirtIoHal;
use crate::io::block;
use alloc::format;
use log::{error, info};
use virtio_drivers::device::blk::VirtIOBlk;
use virtio_drivers::transport::pci::PciTransport;

/// Initialize a block device which is bound to a PCI Transport.
pub fn init_block_device(transport: PciTransport) {
    let blk = match VirtIOBlk::<QuayVirtIoHal, _>::new(transport) {
        Ok(device) => device,
        Err(e) => {
            error!("Failed to initialize VirtIO Block: {:?}", e);
            return;
        }
    };

    let capacity = blk.capacity();

    // Hand the hardware driver over to the generic IO registry
    let device_id = block::register_device(blk);

    // Calculate UNIX-like name (vda, vdb, vdc...)
    let device_char = (b'a' + device_id as u8) as char;
    let device_name = format!("vd{}", device_char);

    info!(
        "Registered Block Device '/dev/{}' | Capacity: {} sectors ({} MiB)",
        device_name,
        capacity,
        (capacity * 512) / 1024 / 1024
    );

    // Let's do a quick read test on the newly registered device using the IO API
    let mut buffer = alloc::vec![0u8; 512];
    if let Err(e) = block::read_block(device_id, 0, &mut buffer) {
        error!("Failed to read Sector 0 of /dev/{}: {:?}", device_name, e);
    } else if buffer[510] == 0x55 && buffer[511] == 0xAA {
        info!(
            "Sector 0 of '/dev/{}' contains a valid boot signature (0x55AA)!",
            device_name
        );
    }

    // Tell the IO subsystem to parse the partition table!
    block::gpt::scan_partitions(device_id);
}
