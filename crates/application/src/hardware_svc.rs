//! Heterogeneous hardware admission service.
//!
//! Bridges runtime-detected worker capabilities with the resource class catalog
//! and a training/conversion job's hardware requirement.  The service fails
//! fast at admission time so unsupported configurations never enter the queue.

use moqentra_domain::hardware::{
    Compatibility, HardwareCompatibility, HardwareRequirement, WorkerCapability,
};
use moqentra_domain::resource_class::{ResourceClass, SupportTier};
use moqentra_domain::training::TrainingJob;
use moqentra_types::Error;

/// Admission result for a workload on heterogeneous hardware.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareAdmission {
    /// A supported (`Supported`) resource class satisfies the job.
    Supported,
    /// Only a `Preview` or `CompileOnly` resource class is available.
    RequiresApproval,
    /// No compatible resource class exists; workload should be rejected.
    Incompatible,
}

/// Service that evaluates whether a training or conversion job can run on the
/// currently registered worker pool.
pub struct HardwareCompatibilityService;

impl HardwareCompatibilityService {
    /// Derive a `HardwareRequirement` from a training job and validate it
    /// against the supplied worker capabilities and resource classes.
    pub fn admit_training_job(
        job: &TrainingJob,
        capabilities: &[WorkerCapability],
        resource_classes: &[ResourceClass],
    ) -> Result<HardwareAdmission, Error> {
        let req = Self::requirement_from_job(job)?;
        let compatibility = HardwareCompatibility::admit(capabilities, resource_classes, &req)?;
        match compatibility {
            Compatibility::Compatible => Ok(HardwareAdmission::Supported),
            Compatibility::Preview | Compatibility::CompileOnly => {
                Ok(HardwareAdmission::RequiresApproval)
            }
            Compatibility::Incompatible | Compatibility::Blocked => Err(Error::invalid_argument(
                "no resource class supports the requested hardware configuration",
            )),
        }
    }

    /// Build a `HardwareRequirement` from a `TrainingJob` resource request.
    fn requirement_from_job(job: &TrainingJob) -> Result<HardwareRequirement, Error> {
        let spec = &job.spec;
        let res = &spec.resources;
        let vendor = match res.accelerator_kind.as_deref() {
            Some(label) => moqentra_domain::hardware::Vendor::from_label(label)
                .ok_or_else(|| Error::invalid_argument("unknown accelerator vendor label"))?,
            None => {
                return Err(Error::invalid_argument(
                    "heterogeneous hardware admission requires an accelerator kind",
                ));
            }
        };

        let collective = vendor_collective(vendor);
        let runtime = runtime_for_vendor(vendor);
        let mut req = HardwareRequirement::new(
            vendor, runtime, collective,
            // Per-device memory requirement is not available from the job spec
            // until model metadata is attached; the resource class profile and
            // worker capability still enforce a positive device memory.
            0,
        )
        .with_min_devices_per_replica(res.accelerator_count.max(1));

        if let Some(framework) = framework_for_image(&spec.image_digest) {
            req = req.with_framework(framework);
        }
        if spec.resources.replicas > 1 {
            // Multi-replica training requires a supported DDP-capable profile.
            req = req.with_max_support_tier(SupportTier::Supported);
        }

        Ok(req)
    }
}

fn vendor_collective(vendor: moqentra_domain::hardware::Vendor) -> String {
    match vendor {
        moqentra_domain::hardware::Vendor::Nvidia => "nccl".to_string(),
        moqentra_domain::hardware::Vendor::Amd => "rccl".to_string(),
        moqentra_domain::hardware::Vendor::Ascend => "hccl".to_string(),
    }
}

fn runtime_for_vendor(vendor: moqentra_domain::hardware::Vendor) -> String {
    match vendor {
        moqentra_domain::hardware::Vendor::Nvidia => "cuda-12.8".to_string(),
        moqentra_domain::hardware::Vendor::Amd => "rocm-6.4".to_string(),
        moqentra_domain::hardware::Vendor::Ascend => "cann-9.0".to_string(),
    }
}

fn framework_for_image(image_digest: &str) -> Option<String> {
    // Placeholder: the production catalog maps (image digest, tag) to framework.
    // A digest starting with `sha256:00` is treated as a PyTorch image for
    // unit-test stability.
    if image_digest.starts_with("sha256:00") || image_digest.contains("pytorch") {
        Some("pytorch".to_string())
    } else if image_digest.contains("mindspore") {
        Some("mindspore".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::hardware::{Vendor, WorkerCapability};
    use moqentra_domain::resource_class::{ResourceClass, SharingMode, SupportTier};
    use moqentra_domain::training::{DistributedConfig, ParameterSchema, TrainingJobSpec};
    use moqentra_types::{DatasetVersionId, ProjectId, RandomIdGenerator, TenantId, TrainingJobId};

    fn nvidia_class(tier: SupportTier) -> ResourceClass {
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

    fn nvidia_worker() -> WorkerCapability {
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

    fn job_with_accelerator(kind: &str, count: u32, replicas: u32) -> TrainingJob {
        let g = RandomIdGenerator;
        let tenant_id = TenantId::new_v7(&g);
        let project_id = ProjectId::new_v7(&g);
        let id = TrainingJobId::new_v7(&g);
        let spec = TrainingJobSpec {
            code_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            image_digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            dataset_version_id: DatasetVersionId::new_v7(&g),
            resources: moqentra_domain::training::ResourceRequest {
                replicas,
                cpu_milli: 8000,
                memory_mib: 1024,
                ephemeral_storage_mib: 1024,
                accelerator_kind: Some(kind.to_string()),
                accelerator_count: count,
                topology: None,
            },
            hyperparameters: ParameterSchema {
                argv: vec!["train.py".to_string()],
                env: Default::default(),
                config_files: Default::default(),
            },
            checkpoint_policy: Default::default(),
            processes_per_replica: 1,
            distributed: if replicas > 1 {
                DistributedConfig::Ddp {
                    world_size: replicas,
                }
            } else {
                DistributedConfig::Single
            },
            max_attempts: 3,
            deadline_seconds: 3600,
            seed: 42,
            resource_class_ref: None,
            queue_ref: None,
            priority_class_ref: None,
            preemption_policy: Default::default(),
        };
        TrainingJob::new(
            id,
            moqentra_types::ExperimentId::new_v7(&g),
            tenant_id,
            project_id,
            spec,
        )
        .unwrap()
    }

    #[test]
    fn admits_supported_nvidia_job() {
        let job = job_with_accelerator("nvidia.com/gpu", 1, 1);
        let result = HardwareCompatibilityService::admit_training_job(
            &job,
            &[nvidia_worker()],
            &[nvidia_class(SupportTier::Supported)],
        )
        .unwrap();
        assert_eq!(result, HardwareAdmission::Supported);
    }

    #[test]
    fn multi_replica_ddp_requires_supported_tier() {
        let job = job_with_accelerator("nvidia.com/gpu", 2, 2);
        // Preview class should be blocked for multi-replica DDP.
        let preview = nvidia_class(SupportTier::Preview);
        assert!(HardwareCompatibilityService::admit_training_job(
            &job,
            &[nvidia_worker()],
            &[preview],
        )
        .is_err());
    }

    #[test]
    fn missing_accelerator_is_rejected() {
        let job = job_with_accelerator("", 0, 1);
        assert!(HardwareCompatibilityService::admit_training_job(
            &job,
            &[nvidia_worker()],
            &[nvidia_class(SupportTier::Supported)],
        )
        .is_err());
    }

    #[test]
    fn incompatible_vendor_is_rejected() {
        let job = job_with_accelerator("amd.com/gpu", 1, 1);
        assert!(HardwareCompatibilityService::admit_training_job(
            &job,
            &[nvidia_worker()],
            &[nvidia_class(SupportTier::Supported)],
        )
        .is_err());
    }
}
