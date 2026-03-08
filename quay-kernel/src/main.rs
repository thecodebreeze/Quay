#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod drivers;
mod hal;
mod serial;
mod sys;

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{HhdmRequest, MemoryMapRequest, RsdpRequest, StackSizeRequest};
use log::{error, info};

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

/// Request the RSDP (ACPI Root System Description Pointer).
#[used]
#[unsafe(link_section = ".requests")]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::with_revision(5);

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

    // Configure the APIC.
    info!("Configuring the APIC...");
    arch::x86_64::pic::disable_legacy_pic();
    arch::x86_64::apic::load_global_lapic_address(hhdm_offset);
    info!("APIC configured!");

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
