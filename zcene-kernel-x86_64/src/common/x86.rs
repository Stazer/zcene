use x86::cpuid::CpuId;

////////////////////////////////////////////////////////////////////////////////////////////////////

pub fn initial_local_apic_id() -> Option<usize> {
    CpuId::new().get_feature_info().map(|feature_info| feature_info.initial_local_apic_id().into())
}
