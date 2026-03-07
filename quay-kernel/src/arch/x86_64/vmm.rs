use crate::sys::memory::phys_memory_manager::{Order, PMM};
use x86_64::PhysAddr;
use x86_64::structures::paging::{
    FrameAllocator, FrameDeallocator, PhysFrame, Size1GiB, Size2MiB, Size4KiB,
};

/// A wrapper to bridge the platform-agnostic PMM to the x86_64 page allocator.
pub struct X86FrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        PMM.lock()
            .allocate(Order::Order4KiB)
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
    }
}

unsafe impl FrameAllocator<Size2MiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size2MiB>> {
        PMM.lock()
            .allocate(Order::Order2MiB)
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
    }
}

unsafe impl FrameAllocator<Size1GiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size1GiB>> {
        PMM.lock()
            .allocate(Order::Order1GiB)
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
    }
}

impl FrameDeallocator<Size4KiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let phys_start = frame.start_address().as_u64();
        PMM.lock().deallocate(phys_start, Order::Order4KiB);
    }
}

impl FrameDeallocator<Size2MiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size2MiB>) {
        let phys_start = frame.start_address().as_u64();
        PMM.lock().deallocate(phys_start, Order::Order2MiB);
    }
}

impl FrameDeallocator<Size1GiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size1GiB>) {
        let phys_start = frame.start_address().as_u64();
        PMM.lock().deallocate(phys_start, Order::Order1GiB);
    }
}
