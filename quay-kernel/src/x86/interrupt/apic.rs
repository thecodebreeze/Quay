use acpi::platform::interrupt::Apic;
use core::sync::atomic::{AtomicUsize, Ordering};
use log::{error, trace, warn};
use spin::Mutex;
use x2apic::ioapic::{IoApic, IrqFlags, IrqMode, RedirectionTableEntry};
use x2apic::lapic::{LocalApic, LocalApicBuilder, TimerDivide, TimerMode};
use x86_64::structures::idt::InterruptStackFrame;
use x86_64::structures::paging::{
    FrameAllocator, Mapper, Page, PageTableFlags, PhysFrame, Size4KiB, mapper,
};
use x86_64::{PhysAddr, VirtAddr};

/// We want to map our keyboard interrupt to this ID, because it matches the legacy PIC 32 + 1
/// offset.
pub const KEYBOARD_INTERRUPT_ID: u8 = 33;

/// Store the Local APIC globally so our interrupt handlers can send the EOI signals.
pub static LAPIC: Mutex<Option<SafeLocalApic>> = Mutex::new(None);

/// We implement Send and Sync manually. Since we have Mutex to guard the [LocalApic] there's no way
/// a data race is going to happen.
///
/// Trust me bro.
pub struct SafeLocalApic(pub LocalApic);
unsafe impl Send for SafeLocalApic {}
unsafe impl Sync for SafeLocalApic {}

/// Initializes the LAPIC device for the current CPU.
pub fn init_apic(
    apic_info: Apic,
    hhdm_offset: u64,
    ticks_per_ms: u32,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    // Map the Local APIC.
    map_mmio(
        hhdm_offset,
        apic_info.local_apic_address,
        mapper,
        frame_allocator,
    );

    // Map the IOAPIC.
    let io_apic_info = apic_info.io_apics.first().expect("No IOAPIC found!");
    map_mmio(
        hhdm_offset,
        io_apic_info.address as u64,
        mapper,
        frame_allocator,
    );

    // Set up Local APIC (LAPIC) on the CPU Core.
    let lapic_virtual_address = apic_info.local_apic_address + hhdm_offset;
    let mut lapic = LocalApicBuilder::new()
        .timer_vector(32)
        .error_vector(34)
        .spurious_vector(255)
        .set_xapic_base(lapic_virtual_address)
        .build()
        .expect("Failed to initialize Local APIC");

    // Enable the Local APIC and configure the timer.
    let lapic_id = unsafe {
        lapic.enable();

        // Divide the CPU's bus frequency by 16 for the timer.
        lapic.set_timer_divide(TimerDivide::Div16);

        // Set it to automatically restart when it hits 0.
        lapic.set_timer_mode(TimerMode::Periodic);

        // Give it a countdown value.
        //
        // This will need calibration against the RTC or PIT.
        lapic.set_timer_initial(ticks_per_ms);

        lapic.id()
    };

    // Hand ownership to our global mutex.
    *LAPIC.lock() = Some(SafeLocalApic(lapic));
    trace!("Local APIC enabled on CPU Core {}", lapic_id);

    // Setup I/O APIC on the Motherboard.
    let io_apic_info = apic_info.io_apics.first().expect("No IOAPIC found!");
    let io_apic_virtual_address = io_apic_info.address as u64 + hhdm_offset;
    let mut io_apic = unsafe { IoApic::new(io_apic_virtual_address) };
    unsafe { io_apic.init(io_apic_info.global_system_interrupt_base as u8) }

    // Find the Keyboard IRQ (Usually 1, but we check the override table just in case).
    let mut keyboard_gsi = 1;
    for int_override in apic_info.interrupt_source_overrides.iter() {
        if int_override.isa_source == 1 {
            keyboard_gsi = int_override.global_system_interrupt as u8;
            warn!(
                "Motherboard wiring quirk: Keyboard IRQ overridden to GSI {}",
                keyboard_gsi
            );
        }
    }

    // Route the Keyboard IRQ to our Local ACPI.
    let mut entry = RedirectionTableEntry::default();
    entry.set_vector(KEYBOARD_INTERRUPT_ID);
    entry.set_mode(IrqMode::Fixed);
    entry.set_flags(IrqFlags::empty());
    entry.set_dest(lapic_id as u8);

    unsafe {
        io_apic.set_table_entry(keyboard_gsi, entry);
        io_apic.enable_irq(keyboard_gsi);
    }
    trace!(
        "IOAPIC routing configured. Keyboard mapped to GSI {}",
        keyboard_gsi
    );

    // Turn on the CPU interrupts.
    x86_64::instructions::interrupts::enable();
    trace!("CPU interrupts enabled (APIC Mode)!");
}

pub fn create_temp_lapic(phys_addr: u64, hhdm_offset: u64) -> LocalApic {
    LocalApicBuilder::new()
        .timer_vector(32)
        .error_vector(34)
        .spurious_vector(255)
        .set_xapic_base(phys_addr + hhdm_offset)
        .build()
        .expect("Failed to build temporary LAPIC for calibration")
}

pub(crate) extern "x86-interrupt" fn keyboard_interrupt_handler(_stack_frame: InterruptStackFrame) {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };

    // Print the raw scancode to the serial log
    log::info!("KEYBOARD SCANCODE: {}", scancode);

    // Send the "End of Interrupt" signal to the modern Local APIC!
    if let Some(lapic) = LAPIC.lock().as_mut() {
        unsafe {
            lapic.0.end_of_interrupt();
        }
    }
}

/// A simple global counter for our timer ticks
pub static TICKS: AtomicUsize = AtomicUsize::new(0);

pub(crate) extern "x86-interrupt" fn apic_timer_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    // Increment the tick counter safely.
    TICKS.fetch_add(1, Ordering::Relaxed);

    // Acknowledge the interrupt
    if let Some(lapic_wrapper) = LAPIC.lock().as_mut() {
        unsafe {
            lapic_wrapper.0.end_of_interrupt();
        }
    }
}

pub(crate) extern "x86-interrupt" fn apic_error_interrupt_handler(
    _stack_frame: InterruptStackFrame,
) {
    error!("APIC ERROR INTERRUPT FIRED");
    if let Some(lapic_wrapper) = LAPIC.lock().as_mut() {
        unsafe {
            lapic_wrapper.0.end_of_interrupt();
        }
    }
}

pub(crate) extern "x86-interrupt" fn spurious_interrupt_handler(_stack_frame: InterruptStackFrame) {
}

pub(crate) fn map_mmio(
    hhdm_offset: u64,
    physical_address: u64,
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    let virtual_address = physical_address + hhdm_offset;
    let frame = PhysFrame::<Size4KiB>::containing_address(PhysAddr::new(physical_address));
    let page = Page::<Size4KiB>::containing_address(VirtAddr::new(virtual_address));

    // MMIO must be explicitly marked as NO_CACHE so the CPU talks directly to the hardware.
    let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE | PageTableFlags::NO_CACHE;
    unsafe {
        match mapper.map_to(page, frame, flags, frame_allocator) {
            Ok(flusher) => flusher.flush(),
            Err(mapper::MapToError::PageAlreadyMapped(_)) => {
                // If Limine mapped it as Read-Only, forcefully override the flags to Writable!
                mapper
                    .update_flags(page, flags)
                    .expect("Failed to update page flags!")
                    .flush();
            }
            Err(error) => panic!("Failed to map MMIO: {:?}", error),
        }
    }
}
