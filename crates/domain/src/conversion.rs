//! Model conversion, evaluation and promotion domain.

use crate::model_registry::{Artifact, ModelSignature};
use moqentra_types::{
    ConversionJobId, ConversionProfileId, DatasetVersionId, EvaluationRunId, ModelVersionId,
    ProjectId, TenantId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

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

impl fmt::Display for ConversionTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionTarget::TensorRT => write!(f, "TensorRT"),
            ConversionTarget::OpenVINO => write!(f, "OpenVINO"),
            ConversionTarget::Rknn => write!(f, "Rknn"),
            ConversionTarget::Sophon => write!(f, "Sophon"),
            ConversionTarget::AscendOM => write!(f, "AscendOM"),
            ConversionTarget::Onnx => write!(f, "Onnx"),
        }
    }
}

impl FromStr for ConversionTarget {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "TensorRT" => Ok(Self::TensorRT),
            "OpenVINO" => Ok(Self::OpenVINO),
            "Rknn" => Ok(Self::Rknn),
            "Sophon" => Ok(Self::Sophon),
            "AscendOM" => Ok(Self::AscendOM),
            "Onnx" => Ok(Self::Onnx),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown conversion target: {s}"
            ))),
        }
    }
}

/// Support tier for a conversion target.
///
/// * `Verified`   - real export, runtime load and fixture inference are
///   implemented; a successful conversion can be marked as `preview`.
/// * `CompileOnly` - a toolchain can compile the model, but runtime
///   verification is not yet available in this worker.
/// * `Unsupported` - no conversion path is implemented.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConversionSupportTier {
    Verified,
    #[default]
    CompileOnly,
    Unsupported,
}

impl ConversionTarget {
    /// The conversion support matrix for this release. TensorRT/OpenVINO
    /// are compile-only until real fixture inference passes; Rknn/Sophon/
    /// AscendOM are unsupported.
    pub fn support_tier(&self) -> ConversionSupportTier {
        match self {
            Self::Onnx => ConversionSupportTier::Verified,
            Self::TensorRT | Self::OpenVINO => ConversionSupportTier::CompileOnly,
            Self::Rknn | Self::Sophon | Self::AscendOM => ConversionSupportTier::Unsupported,
        }
    }
}

/// Post-processing contract required by the conversion target runtime.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PostprocessContract {
    pub confidence_threshold: Option<f64>,
    pub nms_threshold: Option<f64>,
    pub box_format: Option<String>,
    pub class_map: Vec<String>,
}

/// Toolchain and hardware profile for a conversion target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConversionProfile {
    pub id: ConversionProfileId,
    pub name: String,
    pub target: ConversionTarget,
    pub sdk_version: String,
    pub toolchain_image_digest: String,
    pub target_chip: String,
    pub precision: String,
    pub dynamic_shapes: bool,
    pub capabilities: Vec<String>,
    pub postprocess: Option<PostprocessContract>,
    pub parameter_schema: BTreeMap<String, String>,
    pub support_tier: ConversionSupportTier,
    pub revision: u64,
    pub created_at: UtcTimestamp,
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

impl fmt::Display for ConversionJobState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConversionJobState::Pending => write!(f, "Pending"),
            ConversionJobState::Running => write!(f, "Running"),
            ConversionJobState::Succeeded => write!(f, "Succeeded"),
            ConversionJobState::Failed => write!(f, "Failed"),
            ConversionJobState::Cancelled => write!(f, "Cancelled"),
        }
    }
}

impl FromStr for ConversionJobState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(Self::Pending),
            "Running" => Ok(Self::Running),
            "Succeeded" => Ok(Self::Succeeded),
            "Failed" => Ok(Self::Failed),
            "Cancelled" => Ok(Self::Cancelled),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown conversion job state: {s}"
            ))),
        }
    }
}

/// A model conversion job.
#[derive(Debug, Clone, PartialEq)]
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
    /// `true` only when real export, runtime load and fixture inference passed.
    /// `preview` is never persisted to PostgreSQL in this release; it is kept in
    /// memory for workers/agents that can verify the converted model.
    pub preview: bool,
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
    ) -> Result<Self, moqentra_types::Error> {
        if profile.sdk_version.trim().is_empty()
            || profile.target_chip.trim().is_empty()
            || profile.precision.trim().is_empty()
        {
            return Err(moqentra_types::Error::invalid_argument(
                "conversion profile fields must be non-empty",
            ));
        }
        if !moqentra_types::valid_content_digest(&profile.toolchain_image_digest) {
            return Err(moqentra_types::Error::invalid_argument(
                "toolchain_image_digest must be a valid content digest",
            ));
        }
        let now = UtcTimestamp::now();
        let cache_key = Self::compute_cache_key(&source_model_version_id, &profile, &parameters);
        Ok(Self {
            id,
            tenant_id,
            project_id,
            source_model_version_id,
            target,
            profile,
            parameters,
            state: ConversionJobState::Pending,
            output_artifacts: Vec::new(),
            preview: false,
            cache_key,
            log_digest: None,
            created_at: now,
            updated_at: now,
        })
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
        if let Some(pp) = &profile.postprocess {
            if let Some(v) = pp.confidence_threshold {
                hasher.update(v.to_string().as_bytes());
            }
            if let Some(v) = pp.nms_threshold {
                hasher.update(v.to_string().as_bytes());
            }
            if let Some(v) = &pp.box_format {
                hasher.update(v.as_bytes());
            }
            for c in &pp.class_map {
                hasher.update(c.as_bytes());
            }
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
        verified: bool,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ConversionJobState::Running) {
            return Err(moqentra_types::Error::conflict("conversion not running"));
        }
        if artifacts.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "conversion must produce at least one artifact",
            ));
        }
        if !moqentra_types::valid_content_digest(&log_digest) {
            return Err(moqentra_types::Error::invalid_argument(
                "log_digest must be a valid content digest",
            ));
        }
        for artifact in &artifacts {
            artifact.validate()?;
            if artifact.scan_status != "clean" {
                return Err(moqentra_types::Error::invalid_argument(
                    "output artifact not clean",
                ));
            }
        }

        match self.target.support_tier() {
            ConversionSupportTier::Unsupported => {
                return Err(moqentra_types::Error::invalid_argument(
                    "conversion target is unsupported in this release",
                ));
            }
            ConversionSupportTier::CompileOnly if verified => {
                return Err(moqentra_types::Error::invalid_argument(
                    "compile-only target cannot claim runtime verification (preview)",
                ));
            }
            _ => {}
        }

        self.preview = matches!(
            (self.target.support_tier(), verified),
            (ConversionSupportTier::Verified, true)
        );
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

impl fmt::Display for EvaluationRunState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EvaluationRunState::Pending => write!(f, "Pending"),
            EvaluationRunState::Running => write!(f, "Running"),
            EvaluationRunState::Succeeded => write!(f, "Succeeded"),
            EvaluationRunState::Failed => write!(f, "Failed"),
        }
    }
}

impl FromStr for EvaluationRunState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Pending" => Ok(Self::Pending),
            "Running" => Ok(Self::Running),
            "Succeeded" => Ok(Self::Succeeded),
            "Failed" => Ok(Self::Failed),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown evaluation run state: {s}"
            ))),
        }
    }
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
    ) -> Result<Self, moqentra_types::Error> {
        let hardware_profile = hardware_profile.into();
        let preprocess_version = preprocess_version.into();
        let postprocess_version = postprocess_version.into();
        if hardware_profile.trim().is_empty()
            || preprocess_version.trim().is_empty()
            || postprocess_version.trim().is_empty()
        {
            return Err(moqentra_types::Error::invalid_argument(
                "evaluation run string fields must be non-empty",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            tenant_id,
            project_id,
            model_version_id,
            dataset_version_id,
            seed,
            metrics: Vec::new(),
            state: EvaluationRunState::Pending,
            hardware_profile,
            preprocess_version,
            postprocess_version,
            reference_outputs: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn start(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, EvaluationRunState::Pending) {
            return Err(moqentra_types::Error::conflict("evaluation not pending"));
        }
        self.state = EvaluationRunState::Running;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn report_metrics(
        &mut self,
        metrics: Vec<EvaluationMetric>,
    ) -> Result<(), moqentra_types::Error> {
        for metric in &metrics {
            if !metric.value.is_finite() || !metric.tolerance.is_finite() {
                return Err(moqentra_types::Error::invalid_argument(
                    "metric and tolerance must be finite",
                ));
            }
        }
        self.metrics.extend(metrics);
        self.updated_at = UtcTimestamp::now();
        Ok(())
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
        for (name, (min, tol)) in &self.required_metrics {
            let value = run
                .metrics
                .iter()
                .find(|m| &m.name == name)
                .map(|m| m.value)
                .ok_or_else(|| moqentra_types::Error::invalid_argument("missing metric"))?;
            if !value.is_finite() || !min.is_finite() || !tol.is_finite() {
                return Err(moqentra_types::Error::invalid_argument(
                    "metric and threshold must be finite",
                ));
            }
            let threshold = min - tol;
            if value < threshold {
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
            id: ConversionProfileId::new_v7(&RandomIdGenerator),
            name: "onnx-default".to_string(),
            target: ConversionTarget::Onnx,
            sdk_version: "1.16".to_string(),
            toolchain_image_digest:
                "sha256:7c9bbe5ec9b3fb774e8fa0f54247e93c34ddf8e5d16fe3073420de0ae81a262d"
                    .to_string(),
            target_chip: "x86".to_string(),
            precision: "fp32".to_string(),
            dynamic_shapes: false,
            capabilities: vec!["opset17".to_string()],
            postprocess: None,
            parameter_schema: BTreeMap::new(),
            support_tier: ConversionSupportTier::Verified,
            revision: 1,
            created_at: UtcTimestamp::now(),
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
        .unwrap()
    }

    #[test]
    fn conversion_lifecycle() {
        let mut job = make_job();
        job.start().unwrap();
        let gen = RandomIdGenerator;
        job.complete(
            vec![Artifact {
                asset_id: AssetId::new_v7(&gen),
                digest: "sha256:762069bc07a6e1b5df123a5ae7bd91c10daa04694fbaa17fba0cd6a8dcce8f22"
                    .to_string(),
                size_bytes: 100,
                media_type: "application/octet-stream".to_string(),
                scan_status: "clean".to_string(),
                object_key: None,
            }],
            "sha256:836ff184e7b41b1e13cb5fd89fa1de98dbbab99e9d2918913ff43b86a5c7c213".to_string(),
            true,
        )
        .unwrap();
        assert!(matches!(job.state, ConversionJobState::Succeeded));
        assert!(job.preview);
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
                    digest:
                        "sha256:762069bc07a6e1b5df123a5ae7bd91c10daa04694fbaa17fba0cd6a8dcce8f22"
                            .to_string(),
                    size_bytes: 1,
                    media_type: "application/octet-stream".to_string(),
                    scan_status: "infected".to_string(),
                    object_key: None,
                }],
                "sha256:836ff184e7b41b1e13cb5fd89fa1de98dbbab99e9d2918913ff43b86a5c7c213"
                    .to_string(),
                true,
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
        )
        .unwrap();
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
        ])
        .unwrap();
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
