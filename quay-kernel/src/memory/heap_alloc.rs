use spin::Mutex;
use talc::{ErrOnOom, Span, Talc, Talck};
use x86_64::VirtAddr;
use x86_64::structures::paging::mapper::MapToError;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

/// Initialize Talc as our Global Allocator.
///
/// We wrap it in a spinlock (Mutex), so it is thread-safe and use ErrOnOom because our heap size
/// will be fixed for now.
#[global_allocator]
static ALLOCATOR: Talck<Mutex<()>, ErrOnOom> = Talc::new(ErrOnOom).lock();

/// This address is safe within the upper half, but far enough from our kernel that it can be
/// considered safe.
pub const HEAP_START: usize = 0xFFFF_A000_0000_0000;

/// 128MiB of heap space!
pub const HEAP_SIZE: usize = 128 * 1024 * 1024;

pub fn init_heap_alloc(
    mapper: &mut impl Mapper<Size4KiB>,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) -> Result<(), MapToError<Size4KiB>> {
    // Calculate the range of virtual pages we need to map.
    let page_range = {
        let heap_start = VirtAddr::new(HEAP_START as u64);
        let heap_end = heap_start + HEAP_SIZE as u64 - 1u64;
        let heap_start_page = Page::containing_address(heap_start);
        let heap_end_page = Page::containing_address(heap_end);
        Page::range_inclusive(heap_start_page, heap_end_page)
    };

    // Map each virtual page to a physical frame.
    for page in page_range {
        // Ask the bump allocator for a physical frame.
        let frame = frame_allocator
            .allocate_frame()
            .ok_or(MapToError::FrameAllocationFailed)?;

        // Make the heap both readable and writable.
        let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

        // Tell the CPU's page tables to map this virtual page to this physical frame.
        unsafe {
            mapper.map_to(page, frame, flags, frame_allocator)?.flush();
        }
    }

    // Hand the newly mapped memory over to Talc.
    unsafe {
        let heap_span = Span::from_base_size(HEAP_START as *mut u8, HEAP_SIZE);
        ALLOCATOR
            .lock()
            .claim(heap_span)
            .expect("Failed to claim heap memory!");
    }

    Ok(())
}
