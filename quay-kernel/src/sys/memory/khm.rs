//! Kernel Heap Manager implementation using [talc].
//!
//! This is a very simple allocator for kernel heap using the [talc] crate that provides an
//! easy-to-use allocator.

use crate::sys::memory::pmm::{Order, get_pmm};
use crate::sys::memory::vmm::{MapFlags, PageSize, VirtualMapper, get_vmm};
use core::alloc::Layout;
use log::{error, trace};
use spin::Mutex;
use talc::{OomHandler, Span, Talc, Talck};

/// Global allocator for the kernel.
#[global_allocator]
pub static ALLOCATOR: Talck<Mutex<()>, KernelHeapExpander> =
    Talck::new(Talc::new(KernelHeapExpander {
        current_heap_end: HEAP_START_VIRT,
    }));

/// The virtual address where the kernel heap will start.
///
/// This should be somewhere safe inside the Higher Half, well above the kernel code.
pub const HEAP_START_VIRT: u64 = 0xFFFF_A000_0000_0000;

/// How much memory to map every time the heap runs out of space.
///
/// 64 KiB (16 pages) is a great balance between minimizing OOM calls and saving RAM.
pub const HEAP_EXPANSION_CHUNK: u64 = 64 * 1024;

/// The custom handler that teaches `talc` how to dynamically expand the heap.
pub struct KernelHeapExpander {
    /// Tracks the current virtual end of the heap.
    pub current_heap_end: u64,
}

impl OomHandler for KernelHeapExpander {
    fn handle_oom(talc: &mut Talc<Self>, layout: Layout) -> Result<(), ()> {
        // Calculate how much memory is actually need to fulfill the request.
        // Expand by HEAP_EXPANSION_CHUNK, unless the allocation is massive and needs more.
        let bytes_needed = layout.size().max(HEAP_EXPANSION_CHUNK as usize) as u64;

        // Round up to the nearest 4KiB page boundary.
        let pages_needed = bytes_needed.div_ceil(4096);

        let start_virt = talc.oom_handler.current_heap_end;
        let mut mapped_size = 0;

        let map_flags = MapFlags::PRESENT | MapFlags::WRITABLE | MapFlags::NO_EXECUTE;

        // Request physical memory and map it to the virtual heap space.
        for _ in 0..pages_needed {
            let virt_addr = start_virt.saturating_add(mapped_size);

            // Acquire the PMM lock just to allocate and drop it immediately.
            let phys_addr = match get_pmm().allocate(Order::Order4KiB) {
                Ok(addr) => addr,
                Err(error) => {
                    log::error!("KHM OOM: PMM exhausted while expanding heap: {}", error);
                    return Err(());
                }
            };

            // Map it using the architecture-agnostic VMM trait.
            unsafe {
                if let Err(error) =
                    get_vmm().map_page(virt_addr, phys_addr, PageSize::Size4KiB, map_flags)
                {
                    error!("KHM OOM: VMM failed to map heap page: {}", error);
                    // Critical failure: allocated physical RAM but couldn't map it.
                    let _ = get_pmm().deallocate(phys_addr, Order::Order4KiB);
                    return Err(());
                }
            }

            mapped_size = mapped_size.saturating_add(4096);
        }

        // Update the tracking variable.
        talc.oom_handler.current_heap_end = talc
            .oom_handler
            .current_heap_end
            .saturating_add(mapped_size);

        // Tell Talc that it now has a new contiguous span of memory to use.
        let new_span = Span::new(
            start_virt as *mut u8,
            start_virt.saturating_add(mapped_size) as *mut u8,
        );

        unsafe {
            talc.claim(new_span).map_err(|_| {
                error!("KHM OOM: Talc rejected the newly mapped memory span");
            })?;
        }

        trace!("Kernel Heap expanded by {} bytes", mapped_size);
        Ok(())
    }
}
