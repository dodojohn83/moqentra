//! Heterogeneous hardware capability and compatibility model.
//!
//! R2 supports NVIDIA (CUDA/NCCL), AMD (ROCm/RCCL), and Ascend (CANN/HCCL)
//! workers.  A worker's runtime-detected capabilities are matched against the
//! declared profile of a `ResourceClass` before scheduling.  Unsupported,
//! preview, or compile-only tiers are surfaced explicitly so callers do not
//! mistake "can schedule" for "fully supported".

use crate::resource_class::{ResourceClass, SupportTier};
use moqentra_types::UtcTimestamp;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

/// Normalised hardware vendor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Vendor {
    Nvidia,
    Amd,
    Ascend,
}

impl Vendor {
    pub fn from_label(label: &str) -> Option<Self> {
        let lower = label.to_lowercase();
        if lower.contains("nvidia") || lower.contains("cuda") {
            Some(Vendor::Nvidia)
        } else if lower.contains("amd") || lower.contains("rocm") {
            Some(Vendor::Amd)
        } else if lower.contains("ascend") || lower.contains("cann") || lower.contains("npu") {
            Some(Vendor::Ascend)
        } else {
            None
        }
    }
}

/// Runtime-detected capability of a single worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerCapability {
    pub vendor: Vendor,
    pub family: String,
    pub device_model: String,
    pub driver_version: String,
    pub runtime: String,
    pub runtime_version: String,
    pub collective_backend: String,
    pub device_memory_bytes: u64,
    pub device_count: u32,
    pub frameworks: BTreeSet<String>,
    pub model_formats: BTreeSet<String>,
    pub topology: String,
    pub measured_at: UtcTimestamp,
}

impl WorkerCapability {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        vendor: Vendor,
        family: impl Into<String>,
        device_model: impl Into<String>,
        driver_version: impl Into<String>,
        runtime: impl Into<String>,
        runtime_version: impl Into<String>,
        collective_backend: impl Into<String>,
        device_memory_bytes: u64,
        device_count: u32,
    ) -> Self {
        Self {
            vendor,
            family: family.into(),
            device_model: device_model.into(),
            driver_version: driver_version.into(),
            runtime: runtime.into(),
            runtime_version: runtime_version.into(),
            collective_backend: collective_backend.into(),
            device_memory_bytes,
            device_count,
            frameworks: BTreeSet::new(),
            model_formats: BTreeSet::new(),
            topology: "unknown".to_string(),
            measured_at: UtcTimestamp::now(),
        }
    }

    /// Add a supported framework (case-insensitive canonical form).
    pub fn with_framework(mut self, name: impl Into<String>) -> Self {
        self.frameworks.insert(name.into().to_lowercase());
        self
    }

    /// Add a supported model format.
    pub fn with_model_format(mut self, name: impl Into<String>) -> Self {
        self.model_formats.insert(name.into().to_lowercase());
        self
    }
}

/// Required capability declared by a training or conversion operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HardwareRequirement {
    pub vendor: Vendor,
    pub family: Option<String>,
    pub min_driver_version: Option<String>,
    pub runtime: String,
    pub runtime_version: Option<String>,
    pub collective_backend: String,
    pub min_device_memory_bytes: u64,
    pub min_devices_per_replica: u32,
    pub frameworks: BTreeSet<String>,
    pub max_support_tier: SupportTier,
}

impl HardwareRequirement {
    pub fn new(
        vendor: Vendor,
        runtime: impl Into<String>,
        collective_backend: impl Into<String>,
        min_device_memory_bytes: u64,
    ) -> Self {
        Self {
            vendor,
            family: None,
            min_driver_version: None,
            runtime: runtime.into(),
            runtime_version: None,
            collective_backend: collective_backend.into(),
            min_device_memory_bytes,
            min_devices_per_replica: 1,
            frameworks: BTreeSet::new(),
            max_support_tier: SupportTier::Supported,
        }
    }

    pub fn with_family(mut self, family: impl Into<String>) -> Self {
        self.family = Some(family.into());
        self
    }

    pub fn with_min_driver_version(mut self, version: impl Into<String>) -> Self {
        self.min_driver_version = Some(version.into());
        self
    }

    pub fn with_min_devices_per_replica(mut self, count: u32) -> Self {
        self.min_devices_per_replica = count.max(1);
        self
    }

    pub fn with_framework(mut self, name: impl Into<String>) -> Self {
        self.frameworks.insert(name.into().to_lowercase());
        self
    }

    pub fn with_max_support_tier(mut self, tier: SupportTier) -> Self {
        self.max_support_tier = tier;
        self
    }
}

/// Result of matching a `WorkerCapability` against a `ResourceClass` and a
/// `HardwareRequirement`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Compatibility {
    Compatible,
    Incompatible,
    Preview,
    CompileOnly,
    Blocked,
}

impl Compatibility {
    pub fn is_admissible(&self) -> bool {
        !matches!(self, Compatibility::Incompatible | Compatibility::Blocked)
    }

    pub fn is_runnable(&self) -> bool {
        matches!(self, Compatibility::Compatible | Compatibility::Preview)
    }
}

/// Compatibility evaluator used by the scheduler before workload creation.
pub struct HardwareCompatibility;

impl HardwareCompatibility {
    /// Match a worker's runtime capabilities against the requested requirement
    /// and the resource class profile.
    pub fn evaluate(
        capability: &WorkerCapability,
        resource_class: &ResourceClass,
        requirement: &HardwareRequirement,
    ) -> Compatibility {
        if resource_class.vendor.to_lowercase() != capability.vendor_label()
            || resource_class.vendor.to_lowercase() != requirement.vendor_label()
        {
            return Compatibility::Incompatible;
        }

        let tier = resource_class.support_tier;
        if tier as i32 > requirement.max_support_tier as i32 {
            return Compatibility::Blocked;
        }

        if capability.runtime.to_lowercase() != resource_class.runtime.to_lowercase()
            || capability.runtime.to_lowercase() != requirement.runtime.to_lowercase()
        {
            return Compatibility::Incompatible;
        }

        if capability.collective_backend.to_lowercase()
            != resource_class.collective_backend.to_lowercase()
            || capability.collective_backend.to_lowercase()
                != requirement.collective_backend.to_lowercase()
        {
            return Compatibility::Incompatible;
        }

        if capability.device_memory_bytes < requirement.min_device_memory_bytes
            || resource_class.memory_mib * 1024 * 1024 < requirement.min_device_memory_bytes
        {
            return Compatibility::Incompatible;
        }

        if capability.device_count < requirement.min_devices_per_replica {
            return Compatibility::Incompatible;
        }

        if !requirement.frameworks.is_empty() {
            let missing: Vec<_> = requirement
                .frameworks
                .iter()
                .filter(|f| !capability.frameworks.contains(*f))
                .cloned()
                .collect();
            if !missing.is_empty() {
                return Compatibility::Incompatible;
            }
        }

        if let Some(ref required_family) = requirement.family {
            if !resource_class.family.to_lowercase().contains(&required_family.to_lowercase())
                && !capability.family.to_lowercase().contains(&required_family.to_lowercase())
            {
                return Compatibility::Incompatible;
            }
        }

        match tier {
            SupportTier::Supported => Compatibility::Compatible,
            SupportTier::Preview => Compatibility::Preview,
            SupportTier::CompileOnly => Compatibility::CompileOnly,
            SupportTier::Mock => Compatibility::Blocked,
        }
    }

    /// Fail fast at admission time when no schedulable hardware matches the
    /// requirement.  This prevents unsupported operations from entering an
    /// infinite queue.
    pub fn admit(
        capabilities: &[WorkerCapability],
        resource_classes: &[ResourceClass],
        requirement: &HardwareRequirement,
    ) -> Result<Compatibility, moqentra_types::Error> {
        let mut best = Compatibility::Incompatible;
        for class in resource_classes {
            if class.vendor.to_lowercase() != requirement.vendor_label() {
                continue;
            }
            for cap in capabilities {
                let result = Self::evaluate(cap, class, requirement);
                if result == Compatibility::Compatible {
                    return Ok(Compatibility::Compatible);
                }
                if result.is_admissible() && !best.is_admissible() {
                    best = result;
                }
            }
        }

        if best.is_admissible() {
            Ok(best)
        } else {
            Err(moqentra_types::Error::invalid_argument(
                "no compatible hardware for the requested vendor/runtime/collective",
            ))
        }
    }
}

impl WorkerCapability {
    fn vendor_label(&self) -> String {
        match self.vendor {
            Vendor::Nvidia => "nvidia".to_string(),
            Vendor::Amd => "amd".to_string(),
            Vendor::Ascend => "ascend".to_string(),
        }
    }
}

impl HardwareRequirement {
    fn vendor_label(&self) -> String {
        match self.vendor {
            Vendor::Nvidia => "nvidia".to_string(),
            Vendor::Amd => "amd".to_string(),
            Vendor::Ascend => "ascend".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource_class::SharingMode;
    use moqentra_types::RandomIdGenerator;

    fn nvidia_resource_class(tier: SupportTier) -> ResourceClass {
        let g = RandomIdGenerator;
        ResourceClass::new(
            moqentra_types::ResourceClassId::new_v7(&g),
            "nvidia-a100-80g".to_string(),
            "nvidia".to_string(),
            "a100".to_string(),
            81920,
            "550.90".to_string(),
            "cuda-12.8".to_string(),
            "nccl".to_string(),
            "nvlink".to_string(),
            SharingMode::WholeCard,
            tier,
            1,
        )
        .unwrap()
    }

    fn nvidia_capability() -> WorkerCapability {
        WorkerCapability::new(
            Vendor::Nvidia,
            "a100",
            "a100-sxm4-80gb",
            "550.90",
            "cuda-12.8",
            "12.8",
            "nccl",
            80 * 1024 * 1024 * 1024,
            8,
        )
        .with_framework("pytorch")
    }

    fn nvidia_requirement() -> HardwareRequirement {
        HardwareRequirement::new(Vendor::Nvidia, "cuda-12.8", "nccl", 40 * 1024 * 1024 * 1024)
            .with_framework("pytorch")
            .with_min_devices_per_replica(1)
            .with_max_support_tier(SupportTier::CompileOnly)
    }

    #[test]
    fn supported_is_compatible() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Supported);
        let req = nvidia_requirement();
        assert_eq!(
            HardwareCompatibility::evaluate(&cap, &class, &req),
            Compatibility::Compatible
        );
    }

    #[test]
    fn preview_is_preview() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Preview);
        let req = nvidia_requirement();
        assert_eq!(
            HardwareCompatibility::evaluate(&cap, &class, &req),
            Compatibility::Preview
        );
    }

    #[test]
    fn compile_only_not_runnable() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::CompileOnly);
        let req = nvidia_requirement();
        let result = HardwareCompatibility::evaluate(&cap, &class, &req);
        assert!(result.is_admissible());
        assert!(!result.is_runnable());
    }

    #[test]
    fn mock_is_blocked() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Mock);
        let req = nvidia_requirement();
        assert_eq!(
            HardwareCompatibility::evaluate(&cap, &class, &req),
            Compatibility::Blocked
        );
    }

    #[test]
    fn missing_framework_is_incompatible() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Supported);
        let mut req = nvidia_requirement();
        req.frameworks.insert("jax".to_string());
        assert_eq!(
            HardwareCompatibility::evaluate(&cap, &class, &req),
            Compatibility::Incompatible
        );
    }

    #[test]
    fn admit_finds_compatible() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Supported);
        let req = nvidia_requirement();
        assert_eq!(
            HardwareCompatibility::admit(&[cap], &[class], &req).unwrap(),
            Compatibility::Compatible
        );
    }

    #[test]
    fn admit_rejects_incompatible_vendor() {
        let cap = nvidia_capability();
        let class = nvidia_resource_class(SupportTier::Supported);
        let req =
            HardwareRequirement::new(Vendor::Amd, "rocm-6.4", "rccl", 40 * 1024 * 1024 * 1024);
        assert!(HardwareCompatibility::admit(&[cap], &[class], &req).is_err());
    }
}
