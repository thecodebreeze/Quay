//! The Bitmap Physical Memory Manager (BPMM) is an allocator that implements a fast Bitmap Alloc
//! with a spin Mutex that is highly efficient.
//!
//! This BPMM effectively deprecates the old `frame_alloc` implementation that was a simple Bump
//! Alloc implementation.
//!
//! The BPMM also allows us to allocate big pages (2MiB, 1GiB) and handle contiguous large blocks of
//! memory required for DMA and other driver operations.

use core::{ptr, slice};
use lazy_static::lazy_static;
use limine::memory_map::{Entry, EntryType};
use spin::Mutex;
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, FrameDeallocator, PhysFrame, Size4KiB};

lazy_static! {
    /// Lazily initialized global PMM.
    pub static ref PMM: Mutex<BitmapPMM<'static>> = {
        // Fetch the responses directly from the Memory Map.
        let mmap_response = crate::MEMORY_MAP_REQUEST.get_response().expect("MemoryMap to be present");
        let hhdm_response = crate::HHDM_REQUEST.get_response().expect("HHDM to be present");

        let mmap = mmap_response.entries();
        let hhdm_offset = hhdm_response.offset();

        Mutex::new(BitmapPMM::new(mmap, hhdm_offset))
    };
}

/// Bitmap-based Physical Memory Manager.
pub struct BitmapPMM<'a> {
    bitmap: &'a mut [u64],
    next_free: usize,
    free_frames: usize,
    total_frames: usize,
}

impl<'a> BitmapPMM<'a> {
    /// Explores the memory map handed over by Limine in conjunction with the HHDM offset to find
    /// usable memory regions and build the memory manager.
    pub fn new(memory_map: &'a [&'a Entry], hhdm_offset: u64) -> Self {
        // Find the highest physical memory address to size our bitmap.
        let mut highest_address = 0;
        for region in memory_map {
            let end = region.base + region.length;
            if end > highest_address {
                highest_address = end;
            }
        }

        // Compute how many memory frames we can have. Since we will be use u64 for our bitmap
        // array, we must at least guarantee 1 entry.
        let total_frames = (highest_address as usize) / 4096;
        let bitmap_entries = total_frames.div_ceil(64);
        let bitmap_size_bytes = bitmap_entries * 8;

        // Find a usable memory region large enough to hold our bitmap.
        let mut bitmap_ptr: *mut u64 = ptr::null_mut();
        for region in memory_map {
            if region.entry_type == EntryType::USABLE && region.length >= bitmap_size_bytes as u64 {
                // We use the HHDM offset to get the virtual address we can actually write to.
                bitmap_ptr = (region.base + hhdm_offset) as *mut u64;
                break;
            }
        }

        // If we can't find a usable region, panic!
        if bitmap_ptr.is_null() {
            panic!("Not enough contiguous memory to hold the BPMM!");
        }

        // Create the slice pointing to the bitmap and initialize everything as USED (1).
        let bitmap = unsafe { slice::from_raw_parts_mut(bitmap_ptr, bitmap_entries) };
        bitmap.fill(u64::MAX);

        // Mark all Memory Map USABLE entries as FREE (0).
        let mut free_frames = 0;
        for region in memory_map {
            if region.entry_type == EntryType::USABLE {
                let start_frame = region.base as usize / 4096;
                let end_frame = (region.base + region.length) as usize / 4096;
                for frame in start_frame..end_frame {
                    let idx = frame / 64;
                    let bit = frame % 64;
                    bitmap[idx] &= !(1 << bit);
                    free_frames += 1;
                }
            }
        }

        // Reserve the memory used by the bitmap itself. Otherwise, the PMM will hand out frames
        // containing its own tracking data.
        let bitmap_start_frame = (bitmap_ptr as usize - hhdm_offset as usize) / 4096;
        let bitmap_end_frame = bitmap_start_frame + bitmap_size_bytes.div_ceil(4096);
        for frame in bitmap_start_frame..bitmap_end_frame {
            let idx = frame / 64;
            let bit = frame % 64;
            if (bitmap[idx] & (1 << bit)) == 0 {
                bitmap[idx] |= 1 << bit;
                free_frames -= 1;
            }
        }

        // Reserve the zeroth frame. It's technically usable, but returning a 0x0 pointer often leads
        // to null-pointer bugs or conflicts with legacy structures.
        if (bitmap[0] & 1) == 0 {
            bitmap[0] |= 1;
            free_frames -= 1;
        }

        Self {
            bitmap,
            next_free: 0,
            free_frames,
            total_frames,
        }
    }

    /// Allocates a frame from the BPMM.
    ///
    /// Returns a [Option::None] if there's no memory left to be allocated.
    pub fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Fast-Path Search from the last known free index.
        for i in self.next_free..self.bitmap.len() {
            if let Some(frame_address) = self.find_free_frame_address(i) {
                return Some(PhysFrame::containing_address(frame_address));
            }
        }

        // Slow-Path Search from the beginning.
        for i in 0..self.bitmap.len() {
            if let Some(frame_address) = self.find_free_frame_address(i) {
                return Some(PhysFrame::containing_address(frame_address));
            }
        }

        // Out-of-Memory!
        None
    }

    /// Deallocates a frame marking it free for use.
    ///
    /// We do a preemptive optimization by setting the next_free pointer to the page we just freed
    /// so that we allocate pages faster. The performance impact of the if statement is minimal here
    /// compared to the performance gain due to easier searching.
    pub fn deallocate_frame(&mut self, frame: PhysFrame) {
        let frame_number = (frame.start_address().as_u64() / 4096) as usize;
        let index = frame_number / 64;
        let bit = frame_number % 64;

        // Only free if it was actually used.
        if (self.bitmap[index] & (1 << bit)) != 0 {
            self.bitmap[index] &= !(1 << bit);
            self.free_frames += 1;

            // Move the next_free pointer back so we reuse this memory quickly.
            if index < self.next_free {
                self.next_free = index;
            }
        }
    }

    /// Searches a specific bitmap entry for a FREE (0) page and returns its address.
    fn find_free_frame_address(&mut self, index: usize) -> Option<PhysAddr> {
        if self.bitmap[index] != u64::MAX {
            // Get the index of the first 0-bit in the number.
            let bit = self.bitmap[index].trailing_ones() as usize;
            self.bitmap[index] |= 1 << bit;
            self.next_free = index;
            self.free_frames -= 1;

            let frame_number = (index * 64) + bit;
            let address = PhysAddr::new((frame_number * 4096) as u64);
            return Some(address);
        }

        None
    }
}

/// Wrapper struct we can use to pass around to interface with the BPMM.
pub struct GlobalBitmapPMM;

unsafe impl FrameAllocator<Size4KiB> for GlobalBitmapPMM {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        PMM.lock().allocate_frame()
    }
}

impl FrameDeallocator<Size4KiB> for GlobalBitmapPMM {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        PMM.lock().deallocate_frame(frame);
    }
}
