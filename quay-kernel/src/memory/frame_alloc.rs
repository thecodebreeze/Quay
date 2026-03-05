use limine::memory_map::{Entry, EntryType};
use x86_64::PhysAddr;
use x86_64::structures::paging::{FrameAllocator, PhysFrame, Size4KiB};

/// A Frame Allocator that returns usable memory frames based on the Limine memory map.
pub struct BootInfoFrameAllocator<'a> {
    memory_map: &'a [&'a Entry],
    current_region: usize,
    current_addr: u64,
}

impl<'a> BootInfoFrameAllocator<'a> {
    pub fn init(memory_map: &'a [&'a Entry]) -> Self {
        Self {
            memory_map,
            current_region: 0,
            current_addr: 0,
        }
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator<'_> {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        // Loop through the regions starting from where we left off.
        while self.current_region < self.memory_map.len() {
            let region = self.memory_map[self.current_region];

            // Skip non-usable regions.
            if region.entry_type != EntryType::USABLE {
                self.current_region = self.current_region.saturating_add(1);
                continue;
            }

            // If we just moved to a new region, start at its base address.
            if self.current_addr < region.base {
                self.current_addr = region.base;
            }

            // Align the address up to a strict 4KiB page boundary.
            let aligned_addr = (self.current_addr + 4095) & !4095;

            // Ensure a full 4KiB page actually fits before the end of this region.
            if aligned_addr + 4096 <= region.base + region.length {
                let frame = PhysFrame::containing_address(PhysAddr::new(aligned_addr));

                // Move our marker forward by one page for the next allocation call.
                self.current_addr = aligned_addr + 4096;
                return Some(frame);
            } else {
                // This region is full! Move to the next one.
                self.current_region = self.current_region.saturating_add(1);
            }
        }

        // No physical memory left.
        None
    }
}
