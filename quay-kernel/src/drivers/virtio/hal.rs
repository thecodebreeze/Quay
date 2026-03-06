use crate::HHDM_REQUEST;
use crate::memory::pmm::PMM;
use core::ptr;
use core::ptr::NonNull;
use virtio_drivers::{BufferDirection, Hal, PhysAddr};
use x86_64::VirtAddr;
use x86_64::structures::paging::{PhysFrame, Size4KiB, Translate};

/// Hardware Abstraction Layer for VirtIO drivers in Quay.
pub struct QuayVirtIoHal;

unsafe impl Hal for QuayVirtIoHal {
    fn dma_alloc(pages: usize, _direction: BufferDirection) -> (PhysAddr, NonNull<u8>) {
        let mut pmm = PMM.lock();

        // Allocate contiguous memory frames for DMA.
        // Use unwrap_or_else to provide a highly detailed panic message.
        let start_frame_phys_address = pmm.allocate_contiguous_frames(pages).unwrap_or_else(|| {
            panic!(
                "VirtIO DMA Error: Failed to allocate {} contiguous frames! Total free frames left: {}",
                pages,
                pmm.free_frames()
            );
        });

        // Calculate the virtual address.
        let hhdm_offset = HHDM_REQUEST
            .get_response()
            .expect("HHDM to be present")
            .offset();
        let start_frame_virt_address = start_frame_phys_address.as_u64() + hhdm_offset;

        // Ensure the allocated DMA region is zeroed out as required by VirtIO devices.
        unsafe {
            ptr::write_bytes(start_frame_virt_address as *mut u8, 0, pages * 4096);
        }

        (
            start_frame_phys_address.as_u64(),
            NonNull::new(start_frame_virt_address as *mut u8).unwrap(),
        )
    }

    unsafe fn dma_dealloc(paddr: PhysAddr, _vaddr: NonNull<u8>, pages: usize) -> i32 {
        let mut pmm = PMM.lock();

        for i in 0..pages {
            let frame_addr = x86_64::PhysAddr::new((paddr as u64) + (i as u64) * 4096);
            let frame = PhysFrame::<Size4KiB>::containing_address(frame_addr);
            pmm.deallocate_frame(frame);
        }

        0
    }

    unsafe fn mmio_phys_to_virt(paddr: PhysAddr, size: usize) -> NonNull<u8> {
        let hhdm_offset = HHDM_REQUEST.get_response().unwrap().offset();

        // Explicitly map this MMIO region into our page tables so it is writable and no-cache.
        crate::memory::vmm::map_mmio_range(paddr, size as u64, hhdm_offset);

        NonNull::new((paddr + hhdm_offset) as *mut u8).unwrap()
    }

    unsafe fn share(buffer: NonNull<[u8]>, _direction: BufferDirection) -> PhysAddr {
        let vaddr = buffer.as_ptr() as *mut u8 as u64;

        // Ask the VMM for the true physical address of this virtual memory.
        let mapper = crate::memory::vmm::VMM_MAPPER.lock();
        let phys_addr = mapper
            .translate_addr(VirtAddr::new(vaddr))
            .unwrap_or_else(|| {
                panic!(
                    "VirtIO DMA Error: Buffer at {:#x} is not mapped in the page tables!",
                    vaddr
                );
            });

        phys_addr.as_u64()
    }

    unsafe fn unshare(_paddr: PhysAddr, _buffer: NonNull<[u8]>, _direction: BufferDirection) {
        // No-op without an IOMMU
    }
}
