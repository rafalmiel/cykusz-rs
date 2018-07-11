use raw_cpuid::CpuId;

pub fn has_x2apic() -> bool {
    return CpuId::new().get_feature_info().map_or(false, |f| {
        f.has_x2apic()
    });
}