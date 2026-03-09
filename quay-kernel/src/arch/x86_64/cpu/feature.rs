use raw_cpuid::CpuId;

/// Checks if the CPU supports the TSC (Time Stamp Counter) instruction.
pub fn has_tsc() -> bool {
    CpuId::new()
        .get_feature_info()
        .is_some_and(|info| info.has_tsc())
}

/// Checks if the CPU supports the LAPIC TSC-Deadline mode.
pub fn has_tsc_deadline() -> bool {
    CpuId::new()
        .get_feature_info()
        .is_some_and(|info| info.has_tsc_deadline())
}

/// Checks if the CPU TSC is Invariant (ticks at a constant rate).
pub fn has_tsc_invariant() -> bool {
    CpuId::new()
        .get_advanced_power_mgmt_info()
        .is_some_and(|info| info.has_invariant_tsc())
}
