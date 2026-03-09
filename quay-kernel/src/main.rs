#![no_std]
#![no_main]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

extern crate alloc;

mod arch;
mod drivers;
mod hal;
mod serial;
mod sys;

use core::panic::PanicInfo;
use limine::BaseRevision;
use limine::request::{HhdmRequest, MemoryMapRequest, RsdpRequest, StackSizeRequest};
use log::{debug, error, info};

// ============================================================================
// Bootloader Requests
// ============================================================================

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

// ============================================================================
// Kernel Entry Point
// ============================================================================

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // 1. Verify bootloader compatibility.
    assert!(
        BASE_REVISION.is_supported(),
        "Unsupported Limine bootloader revision"
    );

    // 2. Early Logging Subsystem.
    serial::init_logger();
    info!("========================================");
    info!("             Quay v0.0.1                ");
    info!("========================================");

    // 3. Early CPU Initialization.
    info!("[1/4] Initializing CPU architecture...");
    arch::target::init::initialize_cpu();

    // 4. Extract Bootloader Data.
    info!("[2/4] Extracting bootloader data...");
    let hhdm_offset = HHDM_REQUEST
        .get_response()
        .expect("Bootloader did not provide HHDM offset")
        .offset();

    let rsdp_virt_address = RSDP_REQUEST
        .get_response()
        .expect("Bootloader did not provide RSDP address")
        .address() as u64;

    let memory_map = MEMORY_MAP_REQUEST
        .get_response()
        .expect("Bootloader did not provide Memory Map")
        .entries();

    debug!("  -> HHDM Offset: {:#X}", hhdm_offset);
    debug!("  -> RSDP Virtual Address: {:#X}", rsdp_virt_address);
    debug!("  -> Memory Map entries: {}", memory_map.len());

    // 5. Memory Subsystem Initialization.
    info!("[3/4] Initializing the memory subsystem...");
    sys::memory::pmm::initialize(hhdm_offset, memory_map);
    sys::memory::vmm::initialize(hhdm_offset);
    info!("  -> Memory subsystem initialized!");

    // 6. Hardware Discovery & Initialization.
    info!("[4/4] Discovering and initializing hardware...");
    let rsdp_phys_addr = rsdp_virt_address.saturating_sub(hhdm_offset);

    // Delegate hardware specifics to the active architecture
    arch::target::init::initialize_hardware(rsdp_phys_addr, rsdp_virt_address, hhdm_offset);

    // 7. System Ready!
    info!("========================================");
    info!(" Initialization complete! Quay is alive!");
    info!("========================================");

    arch::target::init::enable_interrupts();
    halt_and_catch_fire();
}

// ============================================================================
// Panic & Halt Handlers
// ============================================================================

/// Custom panic handler.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    error!("========================================");
    error!("             KERNEL PANIC               ");
    error!("========================================");
    error!("Location: {:?}", info.location());
    error!("Message: {}", info.message());
    error!("========================================");
    halt_and_catch_fire();
}

/// Halts the CPU continuously.
fn halt_and_catch_fire() -> ! {
    loop {
        arch::target::init::halt();
    }
}
