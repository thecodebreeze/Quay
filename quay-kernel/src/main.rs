#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod hal;
mod serial;
mod sys;

use crate::sys::memory::pmm::PMM;
use acpi::platform::PciConfigRegions;
use alloc::string::{String, ToString};
use alloc::vec;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::text::Text;
use limine::BaseRevision;
use limine::framebuffer::Framebuffer;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, ModuleRequest, RsdpRequest, StackSizeRequest,
};
use log::{debug, error, info, trace};
use x86_64::VirtAddr;

/// Set the Limine base revision.
/// Without this tag, the bootloader will assume revision 0, which we don't want.
#[used]
#[unsafe(link_section = ".requests")]
static BASE_REVISION: BaseRevision = BaseRevision::with_revision(5);

/// Request a specific default stack size.
#[used]
#[unsafe(link_section = ".requests")]
static STACK_SIZE_REQUEST: StackSizeRequest =
    StackSizeRequest::with_revision(5).with_size(128 * 1024);

/// Request the higher-half direct memory map.
#[used]
#[unsafe(link_section = ".requests")]
static HHDM_REQUEST: HhdmRequest = HhdmRequest::with_revision(5);

/// Request the complete Memory Map.
#[used]
#[unsafe(link_section = ".requests")]
static MEMORY_MAP_REQUEST: MemoryMapRequest = MemoryMapRequest::with_revision(5);

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Verify bootloader compatibility.
    assert!(
        BASE_REVISION.is_supported(),
        "Unsupported Limine bootloader revision"
    );

    // Setup Early Logging.
    serial::init_logger();
    info!("=== Quay v0.0.1 ===");

    // CPU Architecture Initialization.
    // TODO: Handle this per-cpu target.
    info!("Loading the GDT and IDT...");
    arch::x86_64::gdt::load_global_descriptor_table();
    arch::x86_64::idt::load_interrupt_descriptor_table();
    info!("GDT and IDT loaded!");

    // Extract bootloader data.
    let hhdm_offset = HHDM_REQUEST
        .get_response()
        .expect("HHDM to be present")
        .offset();

    let memory_map = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory Map to be present")
        .entries();

    info!("HHDM Offset: {:#X}", hhdm_offset);

    // Memory subsystem initialization.
    info!("Initializing the memory subsystem...");
    sys::memory::pmm::initialize(hhdm_offset, memory_map);
    sys::memory::vmm::initialize(hhdm_offset);
    info!("Memory subsystem initialized!");

    info!("Initialization complete! Quay is up and running!");
    halt_and_catch_fire();
}

/// Custom panic handler. For now, just loops forever until we have proper handling.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Kernel panic: {}", info);
    halt_and_catch_fire();
}

fn halt_and_catch_fire() -> ! {
    x86_64::instructions::interrupts::enable();
    loop {
        x86_64::instructions::hlt();
    }
}
