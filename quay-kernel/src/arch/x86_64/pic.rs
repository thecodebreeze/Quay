//! Legacy PIC utilities.
//!
//! Quay doesn't really use Legacy PIC, but it still needs some specific code. Mainly to make sure
//! the device is disabled.

use log::trace;
use x86_64::instructions::port::Port;

/// Disables the legacy PIC entirely by masking out all interrupts.
pub fn disable_legacy_pic() {
    trace!("Disabling legacy PIC...");
    let mut master_data: Port<u8> = Port::new(0x21);
    let mut slave_data: Port<u8> = Port::new(0xA1);

    // Mask all interrupts on both PICs so that Quay doesn't listen to their events.
    unsafe {
        master_data.write(0xFF);
        slave_data.write(0xFF);
    }
}
