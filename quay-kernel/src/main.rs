#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod graphics;
mod memory;
mod platform;
mod serial;

use crate::graphics::DoubleBuffer;
use crate::platform::acpi::QuayAcpiHandler;
use core::panic::PanicInfo;
use embedded_graphics::Drawable;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb888;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::text::Text;
use limine::BaseRevision;
use limine::framebuffer::Framebuffer;
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
    platform::gdt::init_gdt();
    platform::interrupt::init_idt();

    // Memory System Initialization.
    lazy_static::initialize(&memory::vmm::VMM_MAPPER);
    lazy_static::initialize(&memory::pmm::PMM);

    // Get the HHDM offset from Limine.
    let hhdm_response = HHDM_REQUEST.get_response().expect("HHDM request failed!");
    let hhdm_offset = VirtAddr::new(hhdm_response.offset());

    // Hardware Discovery.
    let (acpi_tables, apic_info) = discover_hardware(hhdm_offset);
    let hpet_physical_address = platform::interrupt::timer::get_hpet_base_addr(&acpi_tables);

    // Map Local APIC
    platform::interrupt::apic::map_mmio(hhdm_offset.as_u64(), apic_info.local_apic_address);

    // Map the HPET MMIO temporarily for calibration.
    platform::interrupt::apic::map_mmio(hhdm_offset.as_u64(), hpet_physical_address as u64);

    // Calibration
    let hpet_virtual_address = (hpet_physical_address as u64 + hhdm_offset.as_u64()) as *mut u64;
    let mut temp_lapic = platform::interrupt::apic::create_temp_lapic(
        apic_info.local_apic_address,
        hhdm_offset.as_u64(),
    );
    let ticks_per_ms =
        platform::interrupt::timer::calibrate_apic_timer(&mut temp_lapic, hpet_virtual_address);
    trace!(
        "APIC timer calibrated. Ticks per millisecond: {}",
        ticks_per_ms
    );

    // Capture the framebuffer.
    let framebuffer = get_framebuffer();
    graphics::log_edid_info(&framebuffer);
    let mut display = DoubleBuffer::new(&framebuffer);

    // Set up a text style using the built-in 10x20 font.
    let text_style = MonoTextStyle::new(
        &embedded_graphics::mono_font::iso_8859_1::FONT_7X14,
        Rgb888::new(198, 208, 245),
    );
    // Clear the display.
    display.clear(Rgb888::new(48, 52, 70)).unwrap();
    Text::new(
        "Welcome to Quay v0.0.1!\nDeveloped by thecodebreeze (https://github.com/thecodebreeze)",
        Point::new(0, 14),
        text_style,
    )
    .draw(&mut display)
    .unwrap();
    display.flush();

    // Final hardware driver initialization.
    platform::interrupt::apic::init_apic(apic_info, hhdm_offset.as_u64(), ticks_per_ms);

    info!("Initialization complete! Quay is up and running!");
    halt_and_catch_fire();
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
        platform::acpi::search_acpi_for_apic(handler, rsdp_phys as usize).expect("No APIC found!");

    (acpi_tables, apic_info)
}

fn get_framebuffer<'a>() -> Framebuffer<'a> {
    // Get the response from the framebuffer request.
    let Some(response) = FRAMEBUFFER_REQUEST.get_response() else {
        halt_and_catch_fire();
    };

    // If we don't have at least one framebuffer available, we bail.
    let Some(framebuffer) = response.framebuffers().next() else {
        halt_and_catch_fire();
    };

    framebuffer
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
