use crate::sys::memory::pmm::{Order, PMM, PmmError};
use crate::sys::memory::vmm::{MapFlags, PageSize, VirtualMapper, VmmError};
use core::fmt::Debug;
use log::error;
use x86_64::structures::paging::PageSize as X86PageSize;
use x86_64::structures::paging::mapper::{
    MapToError, MappedFrame, MapperFlush, TranslateResult, UnmapError,
};
use x86_64::structures::paging::{
    FrameAllocator, FrameDeallocator, Mapper, OffsetPageTable, Page, PageTableFlags, PhysFrame,
    Size1GiB, Size2MiB, Size4KiB, Translate,
};
use x86_64::{PhysAddr, VirtAddr};

/// A wrapper to bridge the platform-agnostic PMM to the x86_64 page allocator.
pub struct X86FrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size4KiB>> {
        PMM.lock()
            .allocate(Order::Order4KiB)
            .inspect_err(|error| {
                error!("Failed to allocate frame (4KiB): {:#?}", error);
            })
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
            .ok()
    }
}

unsafe impl FrameAllocator<Size2MiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size2MiB>> {
        PMM.lock()
            .allocate(Order::Order2MiB)
            .inspect_err(|error| {
                error!("Failed to allocate frame (2MiB): {:#?}", error);
            })
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
            .ok()
    }
}

unsafe impl FrameAllocator<Size1GiB> for X86FrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame<Size1GiB>> {
        PMM.lock()
            .allocate(Order::Order1GiB)
            .inspect_err(|error| {
                error!("Failed to allocate frame (1GiB): {:#?}", error);
            })
            .map(|phys_addr| PhysFrame::containing_address(PhysAddr::new(phys_addr)))
            .ok()
    }
}

impl FrameDeallocator<Size4KiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size4KiB>) {
        let phys_start = frame.start_address().as_u64();
        if let Err(error) = PMM.lock().deallocate(phys_start, Order::Order4KiB) {
            error!("Failed to deallocate frame (4KiB): {:#?}", error);
        }
    }
}

impl FrameDeallocator<Size2MiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size2MiB>) {
        let phys_start = frame.start_address().as_u64();
        if let Err(error) = PMM.lock().deallocate(phys_start, Order::Order2MiB) {
            error!("Failed to deallocate frame (2MiB): {:#?}", error);
        }
    }
}

impl FrameDeallocator<Size1GiB> for X86FrameAllocator {
    unsafe fn deallocate_frame(&mut self, frame: PhysFrame<Size1GiB>) {
        let phys_start = frame.start_address().as_u64();
        if let Err(error) = PMM.lock().deallocate(phys_start, Order::Order1GiB) {
            error!("Failed to deallocate frame (1GiB): {:#?}", error);
        }
    }
}

/// Virtual Memory Mapper.
///
/// This mapper implements the [VirtualMapper] trait and wraps a [OffsetPageTable].
pub struct X86Mapper<'a> {
    inner: OffsetPageTable<'a>,
}

impl<'a> VirtualMapper<'a> for X86Mapper<'a> {
    unsafe fn map_page(
        &mut self,
        virt_addr: u64,
        phys_addr: u64,
        size: PageSize,
        flags: MapFlags,
    ) -> Result<(), VmmError> {
        let mut x86_flags = PageTableFlags::empty();

        // Translate the generic flags to x86 flags.
        if flags.contains(MapFlags::PRESENT) {
            x86_flags |= PageTableFlags::PRESENT;
        }
        if flags.contains(MapFlags::WRITABLE) {
            x86_flags |= PageTableFlags::WRITABLE;
        }
        if flags.contains(MapFlags::USER) {
            x86_flags |= PageTableFlags::USER_ACCESSIBLE;
        }
        if flags.contains(MapFlags::NO_EXECUTE) {
            x86_flags |= PageTableFlags::NO_EXECUTE;
        }
        if flags.contains(MapFlags::NO_CACHE) {
            x86_flags |= PageTableFlags::NO_CACHE | PageTableFlags::WRITE_THROUGH;
        }

        // Instantiate the Frame Allocator and prepare for mapping.
        let mut frame_allocator = X86FrameAllocator;
        let virt_addr = VirtAddr::new(virt_addr);
        let phys_addr = PhysAddr::new(phys_addr);

        // Based on the size of the page, map the page to the appropriate frame.
        match size {
            PageSize::Size4KiB => {
                let page = Page::<Size4KiB>::containing_address(virt_addr);
                let frame = PhysFrame::<Size4KiB>::containing_address(phys_addr);
                let flusher: MapperFlush<Size4KiB> = map_page(
                    &mut self.inner,
                    page,
                    frame,
                    x86_flags,
                    &mut frame_allocator,
                )?;

                // Flush the TLB cache.
                flusher.flush();
            }
            PageSize::Size2MiB => {
                let page = Page::<Size2MiB>::containing_address(virt_addr);
                let frame = PhysFrame::<Size2MiB>::containing_address(phys_addr);
                let flusher: MapperFlush<Size2MiB> = map_page(
                    &mut self.inner,
                    page,
                    frame,
                    x86_flags,
                    &mut frame_allocator,
                )?;

                // Flush the TLB cache.
                flusher.flush();
            }
            PageSize::Size1GiB => {
                let page = Page::<Size1GiB>::containing_address(virt_addr);
                let frame = PhysFrame::<Size1GiB>::containing_address(phys_addr);
                let flusher: MapperFlush<Size1GiB> = map_page(
                    &mut self.inner,
                    page,
                    frame,
                    x86_flags,
                    &mut frame_allocator,
                )?;

                // Flush the TLB cache.
                flusher.flush();
            }
        }

        Ok(())
    }

    fn unmap_page(&mut self, virt_addr: u64) -> Result<u64, VmmError> {
        let virt_addr = VirtAddr::new(virt_addr);

        // Walk the page tables to see exactly how this address is mapped.
        let translate_result = self.inner.translate(virt_addr);
        match translate_result {
            TranslateResult::Mapped { frame, .. } => match frame {
                MappedFrame::Size4KiB(_) => {
                    let page = Page::<Size4KiB>::containing_address(virt_addr);
                    unmap_page(&mut self.inner, page, virt_addr, frame)
                }
                MappedFrame::Size2MiB(_) => {
                    let page = Page::<Size4KiB>::containing_address(virt_addr);
                    unmap_page(&mut self.inner, page, virt_addr, frame)
                }
                MappedFrame::Size1GiB(_) => {
                    let page = Page::<Size4KiB>::containing_address(virt_addr);
                    unmap_page(&mut self.inner, page, virt_addr, frame)
                }
            },
            TranslateResult::NotMapped => Err(VmmError::NotMapped(virt_addr.as_u64())),
            TranslateResult::InvalidFrameAddress(_) => Err(VmmError::HardwareFault(
                "Hardware translation encountered an invalid physical frame",
            )),
        }
    }
}

/// Helper function to safely wrap the memory mapping function.
fn map_page<'a, 'b, T: Debug + X86PageSize>(
    mapper: &'a mut OffsetPageTable<'b>,
    page: Page<T>,
    frame: PhysFrame<T>,
    flags: PageTableFlags,
    frame_allocator: &mut X86FrameAllocator,
) -> Result<MapperFlush<T>, VmmError>
where
    OffsetPageTable<'b>: Mapper<T>,
{
    unsafe {
        mapper
            .map_to(page, frame, flags, frame_allocator)
            .inspect_err(|error| {
                error!(
                    "Failed to map page at {:X?} ({:#?})",
                    page.start_address().as_u64(),
                    error
                );
            })
            .map_err(|error| handle_map_error(page.start_address().as_u64(), error))
    }
}

/// Helper function to unmap a specific memory page.
fn unmap_page(
    mapper: &mut OffsetPageTable,
    page: Page,
    virt_addr: VirtAddr,
    frame: MappedFrame,
) -> Result<u64, VmmError> {
    let (phys_frame, flusher) = mapper
        .unmap(page)
        .inspect_err(|error| {
            error!(
                "Failed to unmap page [{:#?}]: {:X?} -> {:X?} ({:#?})",
                PageSize::Size4KiB,
                virt_addr,
                frame.start_address(),
                error
            );
        })
        .map_err(|error| handle_unmap_error(virt_addr.as_u64(), error))?;

    // Flush the TLB cache.
    flusher.flush();
    Ok(phys_frame.start_address().as_u64())
}

/// Helper function to convert memory mapping errors to VMM errors.
#[inline(always)]
fn handle_map_error<T: X86PageSize>(virt_addr: u64, error: MapToError<T>) -> VmmError {
    match error {
        MapToError::PageAlreadyMapped(_) => VmmError::AlreadyMapped(virt_addr),
        MapToError::FrameAllocationFailed => {
            VmmError::PageTableAllocationFailed(PmmError::OutOfMemory(0))
        }
        MapToError::ParentEntryHugePage => {
            VmmError::HardwareFault("Cannot map a 4K page inside an existing 2M/1G huge page")
        }
    }
}

/// Helper function to convert memory mapping errors to VMM errors.
#[inline(always)]
fn handle_unmap_error(virt_addr: u64, error: UnmapError) -> VmmError {
    match error {
        UnmapError::PageNotMapped => VmmError::NotMapped(virt_addr),
        UnmapError::ParentEntryHugePage => {
            VmmError::HardwareFault("Attempted to unmap a 4K page, but parent is a huge page")
        }
        UnmapError::InvalidFrameAddress(_) => {
            VmmError::HardwareFault("Page table points to an invalid physical address")
        }
    }
}
