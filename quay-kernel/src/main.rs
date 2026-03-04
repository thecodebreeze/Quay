#![no_std]
#![no_main]
#![feature(abi_x86_interrupt)]

mod serial;
mod x86;

use core::arch::asm;
use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{FramebufferRequest, HhdmRequest, MemoryMapRequest, StackSizeRequest};
use log::{error, info, trace};

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

    x86_64::instructions::interrupts::int3();
    trace!("Interrupts checked! Working fine like wine.");

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

    // The kernel must never return. If it does, the CPU will like to execute garbage memory and
    // triple fault.
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
    loop {
        unsafe { asm!("hlt") }
    }
}
