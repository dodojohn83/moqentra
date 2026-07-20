//! Model conversion, evaluation and promotion domain.

use crate::model_registry::{Artifact, ModelSignature};
use moqentra_types::{
    ConversionJobId, DatasetVersionId, EvaluationRunId, ModelVersionId, ProjectId, TenantId,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Target backend for conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionTarget {
    TensorRT,
    OpenVINO,
    Rknn,
    Sophon,
    AscendOM,
    Onnx,
}

/// Toolchain and hardware profile for a conversion target.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionProfile {
    pub target: ConversionTarget,
    pub sdk_version: String,
    pub toolchain_image_digest: String,
    pub target_chip: String,
    pub precision: String,
    pub dynamic_shapes: bool,
    pub capabilities: Vec<String>,
}

/// Conversion job state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConversionJobState {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// A model conversion job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionJob {
    pub id: ConversionJobId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub source_model_version_id: ModelVersionId,
    pub target: ConversionTarget,
    pub profile: ConversionProfile,
    pub parameters: BTreeMap<String, String>,
    pub state: ConversionJobState,
    pub output_artifacts: Vec<Artifact>,
    pub cache_key: String,
    pub log_digest: Option<String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl ConversionJob {
    pub fn new(
        id: ConversionJobId,
        tenant_id: TenantId,
        project_id: ProjectId,
        source_model_version_id: ModelVersionId,
        target: ConversionTarget,
        profile: ConversionProfile,
        parameters: BTreeMap<String, String>,
    ) -> Self {
        let now = UtcTimestamp::now();
        let cache_key = Self::compute_cache_key(&source_model_version_id, &profile, &parameters);
        Self {
            id,
            tenant_id,
            project_id,
            source_model_version_id,
            target,
            profile,
            parameters,
            state: ConversionJobState::Pending,
            output_artifacts: Vec::new(),
            cache_key,
            log_digest: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn compute_cache_key(
        source: &ModelVersionId,
        profile: &ConversionProfile,
        parameters: &BTreeMap<String, String>,
    ) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(source.to_string().as_bytes());
        hasher.update(profile.sdk_version.as_bytes());
        hasher.update(profile.toolchain_image_digest.as_bytes());
        hasher.update(profile.target_chip.as_bytes());
        hasher.update(profile.precision.as_bytes());
        hasher.update(profile.dynamic_shapes.to_string().as_bytes());
        hasher.update(format!("{:?}", profile.target).as_bytes());
        for cap in &profile.capabilities {
            hasher.update(cap.as_bytes());
        }
        for (k, v) in parameters {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }
        format!("cache-{:x}", hasher.finalize())
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ConversionJobState::Pending) {
            return Err(moqentra_types::Error::conflict("conversion not pending"));
        }
        self.state = ConversionJobState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn complete(
        &mut self,
        artifacts: Vec<Artifact>,
        log_digest: String,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ConversionJobState::Running) {
            return Err(moqentra_types::Error::conflict("conversion not running"));
        }
        if artifacts.iter().any(|a| a.scan_status != "clean") {
            return Err(moqentra_types::Error::invalid_argument(
                "output artifact not clean",
            ));
        }
        self.output_artifacts = artifacts;
        self.log_digest = Some(log_digest);
        self.state = ConversionJobState::Succeeded;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn fail(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            ConversionJobState::Succeeded | ConversionJobState::Cancelled
        ) {
            return Err(moqentra_types::Error::conflict("already terminal"));
        }
        self.state = ConversionJobState::Failed;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn cancel(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            ConversionJobState::Succeeded | ConversionJobState::Failed
        ) {
            return Err(moqentra_types::Error::conflict("already terminal"));
        }
        self.state = ConversionJobState::Cancelled;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }
}

/// Evaluation metric snapshot.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvaluationMetric {
    pub name: String,
    pub value: f64,
    pub tolerance: f64,
}

/// Evaluation run state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EvaluationRunState {
    Pending,
    Running,
    Succeeded,
    Failed,
}

/// A model evaluation run.
#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationRun {
    pub id: EvaluationRunId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub model_version_id: ModelVersionId,
    pub dataset_version_id: DatasetVersionId,
    pub seed: u64,
    pub metrics: Vec<EvaluationMetric>,
    pub state: EvaluationRunState,
    pub hardware_profile: String,
    pub preprocess_version: String,
    pub postprocess_version: String,
    pub reference_outputs: BTreeMap<String, Vec<u8>>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl EvaluationRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: EvaluationRunId,
        tenant_id: TenantId,
        project_id: ProjectId,
        model_version_id: ModelVersionId,
        dataset_version_id: DatasetVersionId,
        seed: u64,
        hardware_profile: impl Into<String>,
        preprocess_version: impl Into<String>,
        postprocess_version: impl Into<String>,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            tenant_id,
            project_id,
            model_version_id,
            dataset_version_id,
            seed,
            metrics: Vec::new(),
            state: EvaluationRunState::Pending,
            hardware_profile: hardware_profile.into(),
            preprocess_version: preprocess_version.into(),
            postprocess_version: postprocess_version.into(),
            reference_outputs: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, EvaluationRunState::Pending) {
            return Err(moqentra_types::Error::conflict("evaluation not pending"));
        }
        self.state = EvaluationRunState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn report_metrics(&mut self, metrics: Vec<EvaluationMetric>) {
        self.metrics.extend(metrics);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn complete(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, EvaluationRunState::Running) {
            return Err(moqentra_types::Error::conflict("evaluation not running"));
        }
        self.state = EvaluationRunState::Succeeded;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }
}

/// Promotion policy as data.
#[derive(Debug, Clone, PartialEq)]
pub struct PromotionPolicy {
    pub required_metrics: BTreeMap<String, (f64, f64)>,
    pub require_approval: bool,
    pub require_security_scan: bool,
    pub allowed_targets: Vec<ConversionTarget>,
    pub compatibility_signatures: Vec<ModelSignature>,
}

impl PromotionPolicy {
    pub fn evaluate(
        &self,
        run: &EvaluationRun,
        approved: bool,
        scanned: bool,
    ) -> Result<(), moqentra_types::Error> {
        if self.require_approval && !approved {
            return Err(moqentra_types::Error::permission_denied(
                "approval required",
            ));
        }
        if self.require_security_scan && !scanned {
            return Err(moqentra_types::Error::permission_denied(
                "security scan required",
            ));
        }
        for (name, (min, _tol)) in &self.required_metrics {
            let value = run
                .metrics
                .iter()
                .find(|m| &m.name == name)
                .map(|m| m.value)
                .ok_or_else(|| moqentra_types::Error::invalid_argument("missing metric"))?;
            if value.is_nan() || min.is_nan() || value < *min {
                return Err(moqentra_types::Error::invalid_argument(
                    "metric below threshold",
                ));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{AssetId, RandomIdGenerator};

    fn make_profile() -> ConversionProfile {
        ConversionProfile {
            target: ConversionTarget::Onnx,
            sdk_version: "1.16".to_string(),
            toolchain_image_digest: "sha256:tool".to_string(),
            target_chip: "x86".to_string(),
            precision: "fp32".to_string(),
            dynamic_shapes: false,
            capabilities: vec!["opset17".to_string()],
        }
    }

    fn make_job() -> ConversionJob {
        let gen = RandomIdGenerator;
        ConversionJob::new(
            ConversionJobId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            ModelVersionId::new_v7(&gen),
            ConversionTarget::Onnx,
            make_profile(),
            BTreeMap::new(),
        )
    }

    #[test]
    fn conversion_lifecycle() {
        let mut job = make_job();
        job.start().unwrap();
        let gen = RandomIdGenerator;
        job.complete(
            vec![Artifact {
                asset_id: AssetId::new_v7(&gen),
                digest: "sha256:out".to_string(),
                size_bytes: 100,
                media_type: "application/octet-stream".to_string(),
                scan_status: "clean".to_string(),
            }],
            "sha256:log".to_string(),
        )
        .unwrap();
        assert!(matches!(job.state, ConversionJobState::Succeeded));
    }

    #[test]
    fn dirty_artifact_blocks_completion() {
        let mut job = make_job();
        job.start().unwrap();
        let gen = RandomIdGenerator;
        assert!(job
            .complete(
                vec![Artifact {
                    asset_id: AssetId::new_v7(&gen),
                    digest: "sha256:out".to_string(),
                    size_bytes: 1,
                    media_type: "application/octet-stream".to_string(),
                    scan_status: "infected".to_string(),
                }],
                "sha256:log".to_string(),
            )
            .is_err());
    }

    #[test]
    fn promotion_requires_metrics_and_approval() {
        let gen = RandomIdGenerator;
        let mut run = EvaluationRun::new(
            EvaluationRunId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            ModelVersionId::new_v7(&gen),
            DatasetVersionId::new_v7(&gen),
            42,
            "nvidia-a100",
            "pre-1",
            "post-1",
        );
        run.start().unwrap();
        run.report_metrics(vec![
            EvaluationMetric {
                name: "accuracy".to_string(),
                value: 0.95,
                tolerance: 0.01,
            },
            EvaluationMetric {
                name: "mAP".to_string(),
                value: 0.88,
                tolerance: 0.01,
            },
        ]);
        run.complete().unwrap();

        let mut required = BTreeMap::new();
        required.insert("accuracy".to_string(), (0.90, 0.01));
        required.insert("mAP".to_string(), (0.85, 0.01));
        let policy = PromotionPolicy {
            required_metrics: required,
            require_approval: true,
            require_security_scan: true,
            allowed_targets: vec![ConversionTarget::Onnx],
            compatibility_signatures: vec![],
        };
        assert!(policy.evaluate(&run, true, true).is_ok());
        assert!(policy.evaluate(&run, false, true).is_err());
    }
}
