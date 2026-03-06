use crate::arch::x86_64::apic::get_local_apic;
use alloc::boxed::Box;
use spin::{Mutex, MutexGuard};
use x2apic::lapic::LocalApic;
use x86_64::registers::model_specific::GsBase;
use x86_64::VirtAddr;

/// CPU data used internally for diverse operations.
#[repr(C)]
pub struct CpuLocalData {
    /// The hardware ID of this specific CPU core.
    lapic_id: u32,

    /// This core's exclusive Local APIC controller.
    lapic: Mutex<LocalApic>,
}

impl CpuLocalData {
    /// Initialize and load the CpuLocalData into the GS MSR register.
    pub fn load(tick_rate: u32) {
        // Initialize this core's LAPIC.
        let (lapic_id, lapic) = get_local_apic(tick_rate);

        // Create the instance of this struct on the heap.
        let cpu_data = Box::new(CpuLocalData {
            lapic_id,
            lapic: Mutex::new(lapic),
        });

        // By leaking the box we get a raw pointer that lives forever.
        let cpu_data_ptr = Box::leak(cpu_data) as *mut CpuLocalData;

        // Write the pointer to the active GS Base MSR.
        GsBase::write(VirtAddr::from_ptr(cpu_data_ptr))
    }

    /// Retrieves a mutable reference to the current CPU's local data.
    pub fn current() -> &'static Self {
        let cpu_local_data_ptr = GsBase::read().as_u64() as *const CpuLocalData;
        unsafe { &*cpu_local_data_ptr }
    }

    pub fn lapic_id(&self) -> u32 {
        self.lapic_id
    }

    pub fn lapic(&self) -> MutexGuard<'_, LocalApic> {
        self.lapic.lock()
    }
}
