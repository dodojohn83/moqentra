//! Conversion and evaluation application services.

use moqentra_domain::conversion::{ConversionJob, ConversionJobState, EvaluationRun};
use moqentra_types::{ConversionJobId, Error, EvaluationRunId, ProjectId, TenantId};
use std::collections::BTreeMap;

/// In-memory conversion job registry for unit tests and local dev.
#[derive(Debug, Default)]
pub struct InMemoryConversionRegistry {
    jobs: BTreeMap<ConversionJobId, ConversionJob>,
}

impl InMemoryConversionRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state from durable storage (restart recovery).
    pub fn hydrate(&mut self, jobs: Vec<ConversionJob>) {
        self.jobs.clear();
        for j in jobs {
            self.jobs.insert(j.id, j);
        }
    }

    /// Create a conversion job with tenant isolation.
    pub fn create_job(&mut self, job: ConversionJob) -> Result<ConversionJob, Error> {
        if self.jobs.contains_key(&job.id) {
            return Err(Error::conflict("conversion job already exists"));
        }
        self.jobs.insert(job.id, job.clone());
        Ok(job)
    }

    /// Get a conversion job with tenant/project isolation.
    pub fn get_job(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
    ) -> Result<&ConversionJob, Error> {
        let job = self.jobs.get(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        Ok(job)
    }

    /// List conversion jobs for a tenant/project.
    pub fn list_jobs(&self, tenant_id: TenantId, project_id: ProjectId) -> Vec<&ConversionJob> {
        self.jobs
            .values()
            .filter(|j| j.tenant_id == tenant_id && j.project_id == project_id)
            .collect()
    }

    /// Update a conversion job after tenant/project check.
    pub fn update_job(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
        f: impl FnOnce(&mut ConversionJob) -> Result<(), Error>,
    ) -> Result<ConversionJob, Error> {
        let job = self.jobs.get_mut(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        f(job)?;
        Ok(job.clone())
    }

    /// Delete a conversion job.
    pub fn delete_job(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: ConversionJobId,
    ) -> Result<(), Error> {
        let job = self.jobs.get(&id).ok_or_else(|| Error::not_found("conversion job"))?;
        if job.tenant_id != tenant_id || job.project_id != project_id {
            return Err(Error::not_found("conversion job"));
        }
        self.jobs.remove(&id);
        Ok(())
    }

    /// Find a completed successful conversion job with the same cache key.
    /// Used to avoid re-running identical source/profile/parameter combinations.
    pub fn find_successful_by_cache_key(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
        cache_key: &str,
    ) -> Option<&ConversionJob> {
        self.jobs.values().find(|j| {
            j.tenant_id == tenant_id
                && j.project_id == project_id
                && j.cache_key == cache_key
                && j.state == ConversionJobState::Succeeded
        })
    }
}

/// In-memory evaluation run registry.
#[derive(Debug, Default)]
pub struct InMemoryEvaluationRegistry {
    runs: BTreeMap<EvaluationRunId, EvaluationRun>,
}

impl InMemoryEvaluationRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace in-memory state from durable storage (restart recovery).
    pub fn hydrate(&mut self, runs: Vec<EvaluationRun>) {
        self.runs.clear();
        for r in runs {
            self.runs.insert(r.id, r);
        }
    }

    /// Create an evaluation run with tenant/project isolation.
    pub fn create_run(&mut self, run: EvaluationRun) -> Result<EvaluationRun, Error> {
        if self.runs.contains_key(&run.id) {
            return Err(Error::conflict("evaluation run already exists"));
        }
        self.runs.insert(run.id, run.clone());
        Ok(run)
    }

    /// Get an evaluation run.
    pub fn get_run(
        &self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
    ) -> Result<&EvaluationRun, Error> {
        let run = self.runs.get(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        Ok(run)
    }

    /// List evaluation runs.
    pub fn list_runs(&self, tenant_id: TenantId, project_id: ProjectId) -> Vec<&EvaluationRun> {
        self.runs
            .values()
            .filter(|r| r.tenant_id == tenant_id && r.project_id == project_id)
            .collect()
    }

    /// Update an evaluation run.
    pub fn update_run(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
        f: impl FnOnce(&mut EvaluationRun) -> Result<(), Error>,
    ) -> Result<EvaluationRun, Error> {
        let run = self.runs.get_mut(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        f(run)?;
        Ok(run.clone())
    }

    /// Delete an evaluation run.
    pub fn delete_run(
        &mut self,
        tenant_id: TenantId,
        project_id: ProjectId,
        id: EvaluationRunId,
    ) -> Result<(), Error> {
        let run = self.runs.get(&id).ok_or_else(|| Error::not_found("evaluation run"))?;
        if run.tenant_id != tenant_id || run.project_id != project_id {
            return Err(Error::not_found("evaluation run"));
        }
        self.runs.remove(&id);
        Ok(())
    }
}

/// Admission and deduplication service for model conversion jobs.
///
/// Enforces the checks required by R2-CONVERT-004 before a converter is
/// scheduled: source artifact integrity, profile support tier, target runtime
/// capability and required approval for compile-only targets.
pub struct ConversionService;

impl ConversionService {
    /// Check whether a conversion job is admissible for a given source model
    /// artifact and optional target worker capability.
    ///
    /// `approval` must be `Approved` when the profile target is compile-only
    /// and the policy requires explicit sign-off; for verified targets it may
    /// be omitted.
    pub fn admit(
        job: &ConversionJob,
        source_artifact: &moqentra_domain::model_registry::Artifact,
        source_signature: &moqentra_domain::model_registry::ModelSignature,
        target_capability: Option<&moqentra_domain::hardware::WorkerCapability>,
        approval: Option<&moqentra_domain::approval::ApprovalRequest>,
    ) -> Result<(), Error> {
        // Source artifact must be present and clean.
        if source_artifact.scan_status != "clean" {
            return Err(Error::invalid_argument("source artifact scan is not clean"));
        }

        // A conversion must have a non-empty signature to validate compatibility.
        if source_signature.inputs.is_empty() && source_signature.outputs.is_empty() {
            return Err(Error::invalid_argument("source model signature is empty"));
        }

        match job.profile.target.support_tier() {
            moqentra_domain::conversion::ConversionSupportTier::Unsupported => {
                return Err(Error::invalid_argument(
                    "conversion target is unsupported in this release",
                ));
            }
            moqentra_domain::conversion::ConversionSupportTier::CompileOnly => {
                let approval = approval.ok_or_else(|| {
                    Error::permission_denied("compile-only conversion requires approval")
                })?;
                if approval.state != moqentra_domain::approval::ApprovalState::Approved
                    || approval.kind != moqentra_domain::approval::ApprovalKind::ModelPromotion
                {
                    return Err(Error::permission_denied(
                        "compile-only conversion requires an approved model promotion request",
                    ));
                }
            }
            moqentra_domain::conversion::ConversionSupportTier::Verified => {}
        }

        // If the target hardware capability is supplied, require the worker to
        // advertise the target format and the matching runtime.
        if let Some(cap) = target_capability {
            let target_label = job.profile.target.to_string().to_lowercase();
            if !cap.model_formats.contains(&target_label) {
                return Err(Error::invalid_argument(
                    "target worker does not support the requested model format",
                ));
            }
            if !cap.frameworks.contains(&job.profile.sdk_version.to_lowercase()) {
                return Err(Error::invalid_argument(
                    "target worker does not provide the required SDK/framework",
                ));
            }
        }

        // Parameter schema values must be non-empty and free of shell metachars.
        for (k, v) in &job.profile.parameter_schema {
            if k.trim().is_empty() || v.trim().is_empty() {
                return Err(Error::invalid_argument(
                    "conversion profile parameter keys and values must be non-empty",
                ));
            }
            if v.chars().any(|c| matches!(c, ';' | '&' | '|' | '$' | '`' | '\n' | '\r')) {
                return Err(Error::invalid_argument(
                    "conversion profile parameter value contains forbidden shell characters",
                ));
            }
        }

        Ok(())
    }

    /// Look up an existing successful conversion for the same source, profile
    /// and parameters.  Returns the cached job so the scheduler can reuse its
    /// artifacts instead of spawning a duplicate converter.
    pub fn deduplicate<'a>(
        registry: &'a InMemoryConversionRegistry,
        tenant_id: TenantId,
        project_id: ProjectId,
        cache_key: &str,
    ) -> Option<&'a ConversionJob> {
        registry.find_successful_by_cache_key(tenant_id, project_id, cache_key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_domain::approval::{ApprovalKind, ApprovalRequest, ApprovalState};
    use moqentra_domain::conversion::{
        ConversionJobState, ConversionProfile, ConversionSupportTier, ConversionTarget,
    };
    use moqentra_domain::hardware::{Vendor, WorkerCapability};
    use moqentra_domain::model_registry::{Artifact, ModelSignature, TensorSpec};
    use moqentra_types::{
        ApprovalRequestId, AssetId, ConversionJobId, ConversionProfileId, ModelVersionId,
        ProjectId, RandomIdGenerator, TenantId, UserId, UtcTimestamp,
    };

    fn ids() -> (TenantId, ProjectId, ModelVersionId, ConversionJobId) {
        let g = moqentra_types::RandomIdGenerator;
        (
            TenantId::new_v7(&g),
            ProjectId::new_v7(&g),
            ModelVersionId::new_v7(&g),
            ConversionJobId::new_v7(&g),
        )
    }

    fn profile() -> ConversionProfile {
        ConversionProfile {
            id: ConversionProfileId::new_v7(&RandomIdGenerator),
            name: "onnx-default".to_string(),
            target: ConversionTarget::Onnx,
            sdk_version: "1.16.3".to_string(),
            toolchain_image_digest:
                "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                    .to_string(),
            target_chip: "generic".to_string(),
            precision: "fp32".to_string(),
            dynamic_shapes: false,
            capabilities: vec![],
            postprocess: None,
            parameter_schema: std::collections::BTreeMap::new(),
            support_tier: ConversionSupportTier::Verified,
            revision: 1,
            created_at: UtcTimestamp::now(),
        }
    }

    fn job(
        id: ConversionJobId,
        tenant_id: TenantId,
        project_id: ProjectId,
        model_version_id: ModelVersionId,
    ) -> ConversionJob {
        ConversionJob::new(
            id,
            tenant_id,
            project_id,
            model_version_id,
            ConversionTarget::Onnx,
            profile(),
            BTreeMap::new(),
        )
        .unwrap()
    }

    fn source_artifact() -> Artifact {
        let g = RandomIdGenerator;
        Artifact {
            asset_id: AssetId::new_v7(&g),
            digest: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .to_string(),
            size_bytes: 1024,
            media_type: "application/octet-stream".to_string(),
            scan_status: "clean".to_string(),
            object_key: None,
        }
    }

    fn source_signature() -> ModelSignature {
        ModelSignature {
            inputs: vec![TensorSpec {
                name: "input".to_string(),
                dtype: "fp32".to_string(),
                shape: vec![
                    "batch".to_string(),
                    "3".to_string(),
                    "224".to_string(),
                    "224".to_string(),
                ],
            }],
            outputs: vec![TensorSpec {
                name: "output".to_string(),
                dtype: "fp32".to_string(),
                shape: vec!["batch".to_string(), "1000".to_string()],
            }],
        }
    }

    fn onnx_capability() -> WorkerCapability {
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
        .with_model_format("onnx")
        .with_framework("1.16.3")
    }

    fn approved_promotion_request() -> ApprovalRequest {
        let g = RandomIdGenerator;
        let requester = UserId::new_v7(&g);
        let approver = UserId::new_v7(&g);
        let tenant_id = TenantId::new_v7(&g);
        let project_id = ProjectId::new_v7(&g);
        let mut req = ApprovalRequest::new(
            ApprovalRequestId::new_v7(&g),
            tenant_id,
            project_id,
            requester,
            ApprovalKind::ModelPromotion,
            "promote tensorrt".to_string(),
            std::collections::BTreeMap::new(),
            1,
            UtcTimestamp::now(),
        )
        .unwrap();
        req.decide(
            approver,
            ApprovalState::Approved,
            "approved".to_string(),
            None,
            std::collections::BTreeMap::new(),
            1,
        )
        .unwrap();
        req
    }

    fn tensorrt_profile() -> ConversionProfile {
        let mut p = profile();
        p.target = ConversionTarget::TensorRT;
        p.sdk_version = "tensorrt-10.0".to_string();
        p.support_tier = ConversionSupportTier::CompileOnly;
        p
    }

    #[test]
    fn in_memory_conversion_registry_lifecycle() {
        let (t, p, m, id) = ids();
        let mut reg = InMemoryConversionRegistry::new();
        let j = job(id, t, p, m);
        reg.create_job(j.clone()).unwrap();
        assert_eq!(reg.get_job(t, p, id).unwrap().id, id);
        reg.update_job(t, p, id, |j| j.start()).unwrap();
        assert_eq!(
            reg.get_job(t, p, id).unwrap().state,
            ConversionJobState::Running
        );
        reg.delete_job(t, p, id).unwrap();
        assert!(reg.get_job(t, p, id).is_err());
    }

    #[test]
    fn admits_verified_onnx_without_approval() {
        let (t, p, m, id) = ids();
        let job = job(id, t, p, m);
        ConversionService::admit(&job, &source_artifact(), &source_signature(), None, None)
            .unwrap();
    }

    #[test]
    fn dirty_source_artifact_rejected() {
        let (t, p, m, id) = ids();
        let job = job(id, t, p, m);
        let mut artifact = source_artifact();
        artifact.scan_status = "infected".to_string();
        assert!(
            ConversionService::admit(&job, &artifact, &source_signature(), None, None).is_err()
        );
    }

    #[test]
    fn compile_only_requires_approval() {
        let (t, p, m, id) = ids();
        let mut job = job(id, t, p, m);
        job.profile = tensorrt_profile();
        assert!(ConversionService::admit(
            &job,
            &source_artifact(),
            &source_signature(),
            None,
            None
        )
        .is_err());
        ConversionService::admit(
            &job,
            &source_artifact(),
            &source_signature(),
            None,
            Some(&approved_promotion_request()),
        )
        .unwrap();
    }

    #[test]
    fn target_capability_must_support_format() {
        let (t, p, m, id) = ids();
        let job = job(id, t, p, m);
        let mut wrong_capability = onnx_capability();
        wrong_capability.model_formats.clear();
        wrong_capability.model_formats.insert("tensorrt".to_string());
        assert!(ConversionService::admit(
            &job,
            &source_artifact(),
            &source_signature(),
            Some(&wrong_capability),
            None,
        )
        .is_err());
    }

    #[test]
    fn deduplicate_reuses_successful_conversion() {
        let (t, p, m, id) = ids();
        let mut reg = InMemoryConversionRegistry::new();
        let j = job(id, t, p, m);
        reg.create_job(j.clone()).unwrap();
        // Simulate a successful completed job.
        reg.update_job(t, p, id, |job| {
            job.start().unwrap();
            job.complete(
                vec![source_artifact()],
                "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                    .to_string(),
                true,
            )
        })
        .unwrap();
        let cached = ConversionService::deduplicate(&reg, t, p, &j.cache_key);
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().id, id);
    }
}
