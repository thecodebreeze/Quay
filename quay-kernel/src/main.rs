#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod memory;
mod serial;
mod x86;

use crate::memory::frame_alloc::BootInfoFrameAllocator;
use crate::x86::acpi::QuayAcpiHandler;
use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{
    FramebufferRequest, HhdmRequest, MemoryMapRequest, RsdpRequest, StackSizeRequest,
};
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
    x86::gdt::init_gdt();
    x86::interrupt::init_idt();

    // Memory System Initialization.
    let hhdm_offset = get_hhdm_offset();
    let mut mapper = memory::mapper::init_mapper(hhdm_offset);
    let mut frame_allocator = init_pmm();

    // Get the HHDM offset from Limine.
    let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM request failed!");
    let hhdm_offset = VirtAddr::new(hhdm_response.offset());
    trace!("HHDM offset: {:#X}", hhdm_offset);

    memory::heap_alloc::init_heap_alloc(&mut mapper, &mut frame_allocator)
        .expect("Heap initialization failed!");
    trace!("Kernel Heap and PMM initialized.");

    // Hardware Discovery.
    let (acpi_tables, apic_info) = discover_hardware(hhdm_offset);
    let hpet_physical_address = x86::interrupt::timer::get_hpet_base_addr(&acpi_tables);

    // Map Local APIC
    x86::interrupt::apic::map_mmio(
        hhdm_offset.as_u64(),
        apic_info.local_apic_address,
        &mut mapper,
        &mut frame_allocator,
    );

    // Map the HPET MMIO temporarily for calibration.
    x86::interrupt::apic::map_mmio(
        hhdm_offset.as_u64(),
        hpet_physical_address as u64,
        &mut mapper,
        &mut frame_allocator,
    );

    // Calibration
    let hpet_virtual_address = (hpet_physical_address as u64 + hhdm_offset.as_u64()) as *mut u64;
    let mut temp_lapic =
        x86::interrupt::apic::create_temp_lapic(apic_info.local_apic_address, hhdm_offset.as_u64());
    info!("Created temporary LAPIC for calibration.");
    let ticks_per_ms =
        x86::interrupt::timer::calibrate_apic_timer(&mut temp_lapic, hpet_virtual_address);
    info!(
        "APIC timer calibrated. Ticks per millisecond: {}",
        ticks_per_ms
    );

    // Clear the screen.
    clear_screen();
    trace!("Screen cleared. Initializing hardware drivers...");

    // Final hardware driver initialization.
    x86::interrupt::apic::init_apic(
        apic_info,
        hhdm_offset.as_u64(),
        ticks_per_ms,
        &mut mapper,
        &mut frame_allocator,
    );

    info!("Initialization complete! Quay is up and running!");
    halt_and_catch_fire();
}

fn get_hhdm_offset() -> VirtAddr {
    let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM request failed!");
    VirtAddr::new(hhdm_response.offset())
}

fn init_pmm<'a>() -> BootInfoFrameAllocator<'a> {
    let memory_map_response = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Memory Map failed!");
    BootInfoFrameAllocator::init(memory_map_response.entries())
}

fn discover_hardware(
    hhdm_offset: VirtAddr,
) -> (
    acpi::AcpiTables<QuayAcpiHandler>,
    acpi::platform::interrupt::Apic,
) {
    let rsdp_response = RSDP_REQUEST.get_response().expect("No ACPI RSDP found!");
    let rsdp_virt = rsdp_response.address() as u64;
    let rsdp_phys = rsdp_virt - hhdm_offset.as_u64();

    let handler = QuayAcpiHandler::new(hhdm_offset.as_u64());

    let acpi_tables = unsafe {
        // from_rsdp expects a physical address
        acpi::AcpiTables::from_rsdp(handler.clone(), rsdp_phys as usize)
            .expect("Failed to parse ACPI")
    };

    let apic_info =
        x86::acpi::search_acpi_for_apic(handler, rsdp_phys as usize).expect("No APIC found!");

    (acpi_tables, apic_info)
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
    x86_64::instructions::interrupts::enable();

    loop {
        x86_64::instructions::hlt();
    }
}
