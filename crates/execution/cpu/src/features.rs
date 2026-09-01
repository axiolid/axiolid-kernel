//! Runtime CPU feature detection for portable binaries.

/// Instruction set selected for specialized kernels.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CpuInstructionSet {
    /// Architecture-portable scalar path.
    Portable,
    /// x86-64 SSE4.2 path.
    Sse42,
    /// x86-64 AVX2 + FMA path.
    Avx2,
    /// x86-64 AVX-512 foundation path.
    Avx512,
    /// AArch64 NEON path.
    Neon,
}

/// Runtime-detected CPU capabilities. No compile-time `target-cpu=native` is
/// required, so one binary remains safe on older machines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuFeatures {
    /// SSE4.2 available.
    pub sse42: bool,
    /// AVX2 and FMA available.
    pub avx2_fma: bool,
    /// AVX-512 foundation and DQ available.
    pub avx512: bool,
    /// AArch64 NEON available.
    pub neon: bool,
}

impl CpuFeatures {
    /// Detect features on the current process host.
    pub fn detect() -> Self {
        let mut features = Self {
            sse42: false,
            avx2_fma: false,
            avx512: false,
            neon: false,
        };
        #[cfg(target_arch = "x86_64")]
        {
            features.sse42 = std::is_x86_feature_detected!("sse4.2");
            features.avx2_fma =
                std::is_x86_feature_detected!("avx2") && std::is_x86_feature_detected!("fma");
            features.avx512 = std::is_x86_feature_detected!("avx512f")
                && std::is_x86_feature_detected!("avx512dq");
        }
        #[cfg(target_arch = "aarch64")]
        {
            features.neon = std::arch::is_aarch64_feature_detected!("neon");
        }
        features
    }

    /// Whether one implementation is safe to execute.
    pub const fn supports(self, instruction_set: CpuInstructionSet) -> bool {
        match instruction_set {
            CpuInstructionSet::Portable => true,
            CpuInstructionSet::Sse42 => self.sse42,
            CpuInstructionSet::Avx2 => self.avx2_fma,
            CpuInstructionSet::Avx512 => self.avx512,
            CpuInstructionSet::Neon => self.neon,
        }
    }

    /// Best compiled-compatible specialization, or portable scalar.
    pub const fn best(self) -> CpuInstructionSet {
        if self.avx512 {
            CpuInstructionSet::Avx512
        } else if self.avx2_fma {
            CpuInstructionSet::Avx2
        } else if self.sse42 {
            CpuInstructionSet::Sse42
        } else if self.neon {
            CpuInstructionSet::Neon
        } else {
            CpuInstructionSet::Portable
        }
    }
}
