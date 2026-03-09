use crate::arch;
use crate::arch::target::port_io;
use crate::hal::timer::SystemTimer;
use core::arch::x86_64::_rdtsc;
use core::ops::{BitAnd, Shl};
use log::{info, warn};
use raw_cpuid::CpuId;

pub struct Tsc {
    /// The frequency of the TSC in Hz (ticks per second).
    frequency_hz: u64,
}

impl Tsc {
    pub fn initialize() -> Self {
        assert!(
            arch::x86_64::cpu::feature::has_tsc_invariant(),
            "Invariant TSC not supported!"
        );

        let frequency_hz =
            Self::get_frequency_from_cpuid().unwrap_or_else(|| Self::calibrate_via_pit());

        info!("TSC Initialized. Frequency: {} Hz", frequency_hz);
        Self { frequency_hz }
    }

    /// Attempts to read the TSC frequency directly from the CPU.
    fn get_frequency_from_cpuid() -> Option<u64> {
        let cpuid = CpuId::new();

        // Try Leaf 0x15 (Time Stamp Counter and Nominal Core Crystal Clock Information).
        if let Some(tsc_info) = cpuid.get_tsc_info() {
            // Some VMs/CPUs return the ratio but lack the nominal frequency in this leaf.
            let numerator = tsc_info.numerator() as u64;
            let denominator = tsc_info.denominator() as u64;
            let clock_hz = tsc_info.nominal_frequency() as u64;

            if denominator != 0 && numerator != 0 {
                let frequency = clock_hz
                    .saturating_mul(numerator)
                    .saturating_div(denominator);

                return Some(frequency);
            }
        }

        // Try Leaf 0x16 (Processor Frequency Information) as a fallback.
        if let Some(processor_frequency_info) = cpuid.get_processor_frequency_info() {
            let base_mhz = processor_frequency_info.processor_base_frequency();
            if base_mhz > 0 {
                return Some((base_mhz as u64).saturating_mul(1_000_000));
            }
        }

        None
    }

    /// Calibrate the TSC using the legacy PIT as a fallback.
    fn calibrate_via_pit() -> u64 {
        warn!("CPUID frequency detection failed. Falling back to PIT calibration.");
        let pit_reload_value: u16 = 11932;
        port_io::write_u8(0x43, 0b00110000); // Channel 0, lobyte/hibyte, Mode 0
        port_io::write_u8(0x40, (pit_reload_value & 0xFF) as u8);
        port_io::write_u8(0x40, (pit_reload_value >> 8) as u8);

        let start_tsc = unsafe { _rdtsc() };
        loop {
            // Read back command
            port_io::write_u8(0x43, 0b11100010);
            let status = port_io::read_u8(0x40);
            if status.bitand(1u8.shl(7)) != 0u8 {
                break;
            }
            core::hint::spin_loop();
        }

        let end_tsc = unsafe { _rdtsc() };
        let ticks_in_10ms = end_tsc.saturating_sub(start_tsc);

        // Apply a scaling up to 1 second.
        ticks_in_10ms * 100
    }
}

impl SystemTimer for Tsc {
    fn nanos_since_boot(&self) -> u64 {
        let current = unsafe { _rdtsc() } as u128;
        current
            .saturating_mul(1_000_000_000)
            .saturating_div(self.frequency_hz as u128) as u64
    }

    fn stall_us(&self, microseconds: u64) {
        let start = unsafe { _rdtsc() };
        let ticks_to_wait = microseconds
            .saturating_mul(self.frequency_hz)
            .saturating_div(1_000_000);

        while unsafe { _rdtsc() }.saturating_sub(start) < ticks_to_wait {
            core::hint::spin_loop();
        }
    }

    fn sleep_ms(&self, milliseconds: u64) {
        self.stall_us(milliseconds.saturating_mul(1_000));
    }
}
