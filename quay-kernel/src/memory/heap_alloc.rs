use crate::memory::pmm::GlobalBitmapPMM;
use crate::memory::vmm::VMM_MAPPER;
use core::alloc::Layout;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;
use talc::{OomHandler, Span, Talc, Talck};
use x86_64::VirtAddr;
use x86_64::structures::paging::{FrameAllocator, Mapper, Page, PageTableFlags, Size4KiB};

/// Initialize Talc as our Global Allocator.
///
/// We wrap it in a spinlock (Mutex), so it is thread-safe and use ErrOnOom because our heap size
/// will be fixed for now.
#[global_allocator]
static ALLOCATOR: Talck<Mutex<()>, DynamicHeap> = Talc::new(DynamicHeap).lock();

/// This address is safe within the upper half, but far enough from our kernel that it can be
/// considered safe.
pub const HEAP_START: u64 = 0xFFFF_A000_0000_0000;

/// Tracks the current end of the heap virtual memory.
pub static HEAP_CURRENT_END: AtomicU64 = AtomicU64::new(HEAP_START);

/// Custom Out-Of-Memory (OOM) handler for the Talc allocator.
pub struct DynamicHeap;

impl OomHandler for DynamicHeap {
    fn handle_oom(talc: &mut Talc<Self>, layout: Layout) -> Result<(), ()> {
        // We want to map at least 64KiB (16 pages) at a time to avoid excessive page faults and
        // locking overhead for tiny allocations.
        //
        // If the requested layout is larger, we allocate enough to fit it.
        let size = layout.size().saturating_add(4096).max(64 * 1024);
        let pages_needed = size.div_ceil(4096);

        // References to our PMM and VMM.
        let mut frame_allocator = GlobalBitmapPMM;
        let mut mapper = VMM_MAPPER.lock();

        // Fetch the current end of our heap and calcualte the new end.
        let current_end = HEAP_CURRENT_END.load(Ordering::Relaxed);
        let mut new_end = current_end;

        for _ in 0..pages_needed {
            // Allocate a physical frame for each page.
            let frame = frame_allocator.allocate_frame().ok_or(())?;

            // Determine the virtual page to map it to.
            let page = Page::<Size4KiB>::containing_address(VirtAddr::new(new_end));

            // Set the page flags to be active and writable.
            let flags = PageTableFlags::PRESENT | PageTableFlags::WRITABLE;

            // Map the frame to the page.
            unsafe {
                mapper
                    .map_to(page, frame, flags, &mut frame_allocator)
                    .map_err(|_| ())?
                    .flush();
            }

            new_end += 4096;
        }

        // Update the global heap end marker.
        HEAP_CURRENT_END.store(new_end, Ordering::Relaxed);

        // Hand the newly mapped memory over to Talc so it can fulfill the allocation.
        unsafe {
            let heap_span =
                Span::from_base_size(current_end as *mut u8, (new_end - current_end) as usize);
            talc.claim(heap_span).map_err(|_| ())?;
        }

        Ok(())
    }
}
