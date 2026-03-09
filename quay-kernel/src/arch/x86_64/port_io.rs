//! Utilities to interact with I/O ports.

use x86_64::instructions::port::{PortReadOnly, PortWriteOnly};

pub fn read_u8(port: u16) -> u8 {
    unsafe { PortReadOnly::<u8>::new(port).read() }
}

pub fn read_u16(port: u16) -> u16 {
    unsafe { PortReadOnly::<u16>::new(port).read() }
}

pub fn read_u32(port: u16) -> u32 {
    unsafe { PortReadOnly::<u32>::new(port).read() }
}

pub fn write_u8(port: u16, value: u8) {
    unsafe { PortWriteOnly::<u8>::new(port).write(value) }
}

pub fn write_u16(port: u16, value: u16) {
    unsafe { PortWriteOnly::<u16>::new(port).write(value) }
}

pub fn write_u32(port: u16, value: u32) {
    unsafe { PortWriteOnly::<u32>::new(port).write(value) }
}
