//! Buddy Allocator implementation for allocating physical pages of memory.
//!
//! This implementation allows for `Huge Pages` up to `1GiB`.
pub mod error;
pub mod order;

pub use error::PmmError;
pub use order::Order;

use crate::HHDM_REQUEST;
use core::ops::{BitAnd, BitXor, Shl};
use core::ptr;
use lazy_static::lazy_static;
use limine::memory_map::{Entry, EntryType};
use spin::{Mutex, MutexGuard};

lazy_static! {
    pub static ref PMM: Mutex<PhysMemoryManager> = {
        let hhdm_offset = HHDM_REQUEST
            .get_response()
            .expect("HHDM to be present")
            .offset();

        Mutex::new(PhysMemoryManager::new(hhdm_offset))
    };
}

/// The PMM supports every power of 2 page size between 0 (4KiB) and 18 (1GiB).
pub const MAX_ORDER: usize = 19;

/// The default page size for the PMM (4KiB).
pub const DEFAULT_PAGE_SIZE: usize = 0x1000;

// Standard Big Page (2MiB).
pub const DEFAULT_BIG_PAGE_SIZE: usize = 0x200000;

/// The Physical Memory Manager that implements a Buddy Allocator.
///
/// Supports every power of 2 page size between 4KiB and 1GiB.
pub struct PhysMemoryManager {
    /// Array of linked lists, one for each Order of memory.
    free_lists: [*mut FreeBlock; MAX_ORDER],

    /// The HHDM offset, needed so the PMM can write the [FreeBlock] pointers into physical memory
    /// using their virtual higher-half addresses.
    hhdm_offset: u64,

    /// Total memory available to the PMM in bytes.
    total_memory_bytes: u64,

    /// Total memory currently in usage by the PMM in bytes.
    free_memory_bytes: u64,
}

// Ensure that the PMM is thread-safe for the global Mutex.
unsafe impl Send for PhysMemoryManager {}
unsafe impl Sync for PhysMemoryManager {}

/// An intrusive linked list node that lives INSIDE the free physical memory.
struct FreeBlock {
    /// Pointer to the next node.
    next: *mut FreeBlock,
}

impl PhysMemoryManager {
    /// Create a new instance of the PMM.
    ///
    /// Supports creation in `const` contexts.
    pub const fn new(hhdm_offset: u64) -> Self {
        Self {
            free_lists: [ptr::null_mut(); MAX_ORDER],
            hhdm_offset,
            total_memory_bytes: 0,
            free_memory_bytes: 0,
        }
    }

    pub fn init<'a>(&mut self, memory_map: &'a [&'a Entry]) {
        for &region in memory_map {
            // Only care about USABLE regions.
            if region.entry_type != EntryType::USABLE {
                continue;
            }

            // Align the base address UP to the nearest 4KiB page boundary.
            let mut current_base = region.base.saturating_add(4095).bitand(!4095);

            // Align the end address DOWN to the nearest 4KiB page boundary.
            let end_addr = region.base.saturating_add(region.length).bitand(!4095);

            // Carve the region into the largest possible power-of-2 blocks.
            while current_base < end_addr {
                let mut order = MAX_ORDER.saturating_sub(1);
                loop {
                    let block_size = DEFAULT_PAGE_SIZE.shl(order) as u64;

                    // Check if this order satisfies BOTH size and alignment requirements.
                    let fits_in_region = current_base.saturating_add(block_size) <= end_addr;
                    let is_aligned = current_base % block_size == 0;
                    if fits_in_region && is_aligned {
                        // Found the perfect fit.
                        break;
                    }

                    // If it doesn't fit or align, drop down an order and try again.
                    order = order.saturating_sub(1);
                }

                // Push this perfectly aligned chunk into our free lists.
                self.push_to_list(current_base, order);

                // Update the tracking numbers.
                let allocated_size = DEFAULT_PAGE_SIZE.shl(order) as u64;
                self.total_memory_bytes = self.total_memory_bytes.saturating_add(allocated_size);
                self.free_memory_bytes = self.free_memory_bytes.saturating_add(allocated_size);

                // Move the base pointer forward by the size that was carved out.
                current_base = current_base.saturating_add(allocated_size);
            }
        }
    }

    /// Allocates a contiguous block of memory of the specified `order`.
    ///
    /// Returns the physical address of the block we found as a `u64`.
    pub fn allocate(&mut self, order: Order) -> Result<u64, PmmError> {
        // If the order is not supported, return None.
        let order = order as usize;
        if order >= MAX_ORDER {
            return Err(PmmError::OrderTooLarge(order, MAX_ORDER));
        }

        // Find the smallest available order that is >= the requested order.
        for current_order in order..MAX_ORDER {
            let list_head = self.free_lists[current_order];
            if !list_head.is_null() {
                // We found a block! Pop it off the list.
                unsafe {
                    self.free_lists[current_order] = (*list_head).next;
                }

                // If the block is larger than requested, split it down.
                let phys_addr = self.virt_to_phys(list_head as u64);
                self.split_block(phys_addr, current_order, order);

                let allocated_memory = DEFAULT_PAGE_SIZE.shl(order) as u64;
                self.free_memory_bytes = self.free_memory_bytes.saturating_sub(allocated_memory);

                return Ok(phys_addr);
            }
        }

        // No blocks were found. The system is out of memory.
        Err(PmmError::OutOfMemory(order))
    }

    /// Deallocates a contiguous block of memory of the specified `order`.
    pub fn deallocate(&mut self, phys_addr: u64, order: Order) -> Result<(), PmmError> {
        // NO_OP if the order is not supported.
        let order = order as usize;
        if order >= MAX_ORDER {
            return Err(PmmError::OrderTooLarge(order, MAX_ORDER));
        }

        let mut current_order = order;
        let mut current_phys_addr = phys_addr;

        // Try to merge the block up to the maximum possible order.
        while current_order < MAX_ORDER - 1 {
            // Find the buddy's address by flipping the bit corresponding to the block's size.
            let block_size = (DEFAULT_PAGE_SIZE as u64).shl(current_order);
            let buddy_phys_addr = current_phys_addr.bitxor(block_size);

            // Check if the buddy is currently free in the list for this order.
            if self.remove_from_list(buddy_phys_addr, current_order) {
                // The buddy was free. Merge them!
                //
                // The physical address of the merged block is always the lower of the two
                // addresses.
                if buddy_phys_addr < current_phys_addr {
                    current_phys_addr = buddy_phys_addr;
                }

                current_order = current_order.saturating_add(1);
            } else {
                // The buddy is currently in use (or split). Not possible to merge any further.
                break;
            }
        }

        // Push the final (possibly heavily merged) block into the appropriate list.
        self.push_to_list(current_phys_addr, current_order);

        // Update the tracking stats.
        let freed_memory = DEFAULT_PAGE_SIZE.shl(order) as u64;
        self.free_memory_bytes = self.free_memory_bytes.saturating_add(freed_memory);

        Ok(())
    }

    /// Converts a physical address to a virtual address using the HHDM offset.
    #[inline(always)]
    pub fn virt_to_phys(&self, virt_addr: u64) -> u64 {
        virt_addr.saturating_sub(self.hhdm_offset)
    }

    /// Converts a virtual address to a physical address using the HHDM offset.
    #[inline(always)]
    pub fn phys_to_virt(&self, phys_addr: u64) -> u64 {
        phys_addr.saturating_add(self.hhdm_offset)
    }

    /// Pushes a free physical block onto the appropriate free list for its order.
    fn push_to_list(&mut self, phys_addr: u64, order: usize) {
        let virt_addr = self.phys_to_virt(phys_addr);
        let free_block_ptr = virt_addr as *mut FreeBlock;

        unsafe {
            // Points this block's `next` to the current head of the list.
            (*free_block_ptr).next = self.free_lists[order];
        }

        // Update the head of the list to point to this block.
        self.free_lists[order] = free_block_ptr;
    }

    /// Splits a block of `current_order` down to the `target_order`.
    ///
    fn split_block(&mut self, phys_addr: u64, current_order: usize, target_order: usize) {
        let mut order = current_order;

        // Keep splitting the block down until we reach the target order.
        while order > target_order {
            // Step down one order (divide the size by 2).
            order -= 1;

            // Calculate the size of the new smaller blocks.
            let step_size = (DEFAULT_PAGE_SIZE as u64).shl(order);

            // The buddy's physical address is exactly halfway through the current block.
            let buddy_phys_addr = phys_addr.saturating_add(step_size);

            // Push the right-half buddy into the free list for this smaller order.
            self.push_to_list(buddy_phys_addr, order);
        }
    }

    fn remove_from_list(&mut self, target_phys_addr: u64, order: usize) -> bool {
        let target = self.phys_to_virt(target_phys_addr) as *mut FreeBlock;
        let mut current = self.free_lists[order];
        let mut prev: *mut FreeBlock = ptr::null_mut();

        while !current.is_null() {
            if current.eq(&target) {
                unsafe {
                    if prev.is_null() {
                        // Removing the head of the list.
                        self.free_lists[order] = (*current).next;
                    } else {
                        // Removing a node from the middle/end of the list.
                        (*prev).next = (*current).next;
                    }
                }

                return true;
            }

            prev = current;
            unsafe {
                current = (*current).next;
            }
        }

        false
    }
}

/// A clean helper function to grab the PMM lock from anywhere in the kernel.
pub fn get_pmm() -> MutexGuard<'static, PhysMemoryManager> {
    PMM.lock()
}
