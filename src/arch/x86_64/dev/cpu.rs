use raw_cpuid::CpuId;

pub fn has_x2apic() -> bool {
    let cpuid = CpuId::new();

    return cpuid.get_feature_info().map_or(false, |f| {
        f.has_x2apic()
    });
}