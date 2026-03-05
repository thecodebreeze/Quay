#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod memory;
mod serial;
mod x86;

use crate::memory::frame_alloc::BootInfoFrameAllocator;
use crate::x86::acpi::QuayAcpiHandler;
use core::arch::asm;
use core::panic::PanicInfo;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RsdpRequest, StackSizeRequest,
};
use limine::BaseRevision;
use log::{error, info, trace};
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

/// Requests the Root System Descriptor Pointer (RSDP). We use it to find the ACPI table.
#[used]
#[unsafe(link_section = ".requests")]
static RSDP_REQUEST: RsdpRequest = RsdpRequest::with_revision(5);

/// Request a framebuffer for graphics output.
#[used]
#[unsafe(link_section = ".requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::with_revision(5);

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // Initialization sequence start!
    serial::init_logger();
    info!("Quay is booted up!");

    x86::gdt::init_gdt();
    trace!("GDT and TSS loaded successfully!");

    x86::interrupt::init_idt();
    trace!("IDT loaded successfully!");

    // Get the HHDM offset from Limine.
    let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM request failed!");
    let hhdm_offset = VirtAddr::new(hhdm_response.offset());
    trace!("HHDM offset: {:#X}", hhdm_offset);

    // Initialize the Page Table Mapper.
    let mut mapper = memory::mapper::init_mapper(hhdm_offset);
    trace!("Page Table Mapper initialized successfully!");

    // Fetch the memory map so we know how much RAM we actually have.
    let memory_map_response = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory Map request failed!");
    trace!(
        "Detected {} memory map entries.",
        memory_map_response.entries().len()
    );

    // Create the Frame Allocator.
    let mut frame_allocator = BootInfoFrameAllocator::init(memory_map_response.entries());
    trace!("Physical Memory Manager (PMM) initialized successfully!");

    // Initialize the kernel heap.
    memory::heap_alloc::init_heap_alloc(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed!");
    trace!("Kernel Heap initialized successfully!");

    // Load the ACPI Handler.
    let Some(rsdp_response) = RSDP_REQUEST.get_response() else {
        error!("No ACPI RSDP found! Cannot configure the APIC!");
        halt_and_catch_fire();
    };

    let rsdp_addr = rsdp_response.address();
    trace!("ACPI RSDP address: {:#X}", rsdp_addr);

    let rsdp_physical_address = rsdp_addr - hhdm_offset.as_u64() as usize;
    let acpi_handler = QuayAcpiHandler::new(hhdm_offset.as_u64());

    // Try to find a suitable IOAPIC device.
    let Some(apic) = x86::acpi::search_acpi_for_apic(acpi_handler.clone(), rsdp_physical_address)
    else {
        error!("No APIC found! Cannot configure the APIC!");
        halt_and_catch_fire();
    };

    // Initialize the device we found and enable interrupts.
    x86::interrupt::apic::init_apic(apic, hhdm_offset.as_u64(), &mut mapper, &mut frame_allocator);

    // Clear the screen.
    clear_screen();

    // The kernel must never return. If it does, the CPU will like to execute garbage memory and
    // triple fault.
    info!("Initialization complete! Quay is up and running!");
    halt_and_catch_fire();
}

fn clear_screen() {
    // Get the response from the framebuffer request.
    let Some(response) = FRAMEBUFFER_REQUEST.get_response() else {
        halt_and_catch_fire();
    };

    // If we don't have at least one framebuffer available, we bail.
    let Some(framebuffer) = response.framebuffers().next() else {
        halt_and_catch_fire();
    };

    let framebuffer_ptr = framebuffer.addr();
    let width = framebuffer.width() as usize;
    let height = framebuffer.height() as usize;
    let pitch = framebuffer.pitch() as usize;
    let bpp = (framebuffer.bpp() / 8) as usize;

    // Clear the framebuffer.
    for y in 0..height {
        for x in 0..width {
            let pixel_offset = (y * pitch) + (x * bpp);
            // The framebuffer uses BGR rather than RGB!
            unsafe {
                framebuffer_ptr.add(pixel_offset).write_volatile(46);
                framebuffer_ptr.add(pixel_offset + 1).write_volatile(30);
                framebuffer_ptr.add(pixel_offset + 2).write_volatile(30);
            }
        }
    }
}

/// Custom panic handler. For now, just loops forever until we have proper handling.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("Kernel panic: {}", info);
    halt_and_catch_fire();
}

fn halt_and_catch_fire() -> ! {
    loop {
        unsafe { asm!("hlt") }
    }
}
