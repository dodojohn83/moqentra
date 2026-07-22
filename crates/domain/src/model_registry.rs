//! Model registry, artifacts, signatures and lineage.

use moqentra_types::{
    AnnotationProjectId, AssetId, AttemptId, DatasetVersionId, ExperimentId, ModelId,
    ModelVersionId, ProjectId, TenantId, TrainingJobId, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

/// Content-addressed artifact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    pub asset_id: AssetId,
    pub digest: String,
    pub size_bytes: u64,
    pub media_type: String,
    pub scan_status: String,
}

impl Artifact {
    pub(crate) fn validate(&self) -> Result<(), moqentra_types::Error> {
        if self.size_bytes == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact size must be greater than zero",
            ));
        }
        if self.media_type.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact media_type is empty",
            ));
        }
        if !moqentra_types::valid_content_digest(&self.digest) {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact digest must be a valid content digest",
            ));
        }
        if self.scan_status.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact scan_status is empty",
            ));
        }
        const ALLOWED: &[&str] = &["clean", "infected", "pending", "unknown"];
        if !ALLOWED.contains(&self.scan_status.as_str()) {
            return Err(moqentra_types::Error::invalid_argument(
                "artifact scan_status is not a recognized value",
            ));
        }
        Ok(())
    }
}

/// Input/output signature of a model version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelSignature {
    pub inputs: Vec<TensorSpec>,
    pub outputs: Vec<TensorSpec>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TensorSpec {
    pub name: String,
    pub dtype: String,
    pub shape: Vec<String>,
}

/// License or SBOM attachment.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    pub kind: String,
    pub asset_id: AssetId,
}

/// Model version lineage.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelLineage {
    pub training_job_id: Option<TrainingJobId>,
    pub experiment_id: Option<ExperimentId>,
    pub dataset_version_id: DatasetVersionId,
    pub attempt_id: Option<AttemptId>,
    pub annotation_project_id: Option<AnnotationProjectId>,
    pub base_model_version_id: Option<ModelVersionId>,
    pub code_digest: String,
    pub image_digest: String,
    pub hyperparameter_digest: String,
}

/// Model version state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelVersionState {
    Draft,
    Validating,
    Ready,
    Approved,
    Deprecated,
    Rejected,
}

impl FromStr for ModelVersionState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Validating" => Ok(Self::Validating),
            "Ready" => Ok(Self::Ready),
            "Approved" => Ok(Self::Approved),
            "Deprecated" => Ok(Self::Deprecated),
            "Rejected" => Ok(Self::Rejected),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown model version state: {s}"
            ))),
        }
    }
}

/// A model version is an immutable release unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelVersion {
    pub id: ModelVersionId,
    pub model_id: ModelId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub version: String,
    pub state: ModelVersionState,
    pub signature: ModelSignature,
    pub artifacts: Vec<Artifact>,
    pub lineage: ModelLineage,
    pub metrics: BTreeMap<String, f64>,
    pub attachments: Vec<Attachment>,
    pub approved_by: Option<moqentra_types::UserId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl ModelVersion {
    pub fn new(
        id: ModelVersionId,
        model_id: ModelId,
        tenant_id: TenantId,
        project_id: ProjectId,
        version: impl Into<String>,
        lineage: ModelLineage,
    ) -> Result<Self, moqentra_types::Error> {
        let version = version.into();
        if version.trim().is_empty() || version.len() > 64 {
            return Err(moqentra_types::Error::invalid_argument(
                "model version string must be non-empty and at most 64 characters",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            model_id,
            tenant_id,
            project_id,
            version,
            state: ModelVersionState::Draft,
            signature: ModelSignature {
                inputs: Vec::new(),
                outputs: Vec::new(),
            },
            artifacts: Vec::new(),
            lineage,
            metrics: BTreeMap::new(),
            attachments: Vec::new(),
            approved_by: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn validate(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ModelVersionState::Draft) {
            return Err(moqentra_types::Error::conflict(
                "model version not in draft",
            ));
        }
        if self.version.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "model version string is empty",
            ));
        }
        if self.artifacts.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "model version has no artifacts",
            ));
        }
        let mut seen_asset_ids = std::collections::HashSet::new();
        for artifact in &self.artifacts {
            if !seen_asset_ids.insert(artifact.asset_id) {
                return Err(moqentra_types::Error::invalid_argument(
                    "duplicate artifact asset id",
                ));
            }
            artifact.validate()?;
            if artifact.scan_status != "clean" {
                return Err(moqentra_types::Error::invalid_argument(
                    "artifact not clean",
                ));
            }
        }
        if !moqentra_types::valid_content_digest(&self.lineage.code_digest)
            || !moqentra_types::valid_content_digest(&self.lineage.image_digest)
            || !moqentra_types::valid_content_digest(&self.lineage.hyperparameter_digest)
        {
            return Err(moqentra_types::Error::invalid_argument(
                "lineage digests must be valid content digests",
            ));
        }
        self.state = ModelVersionState::Validating;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn mark_ready(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ModelVersionState::Validating) {
            return Err(moqentra_types::Error::conflict(
                "model version not validating",
            ));
        }
        self.state = ModelVersionState::Ready;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn approve(
        &mut self,
        user_id: moqentra_types::UserId,
    ) -> Result<(), moqentra_types::Error> {
        if !matches!(self.state, ModelVersionState::Ready) {
            return Err(moqentra_types::Error::conflict("model version not ready"));
        }
        self.approved_by = Some(user_id);
        self.state = ModelVersionState::Approved;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn reject(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            ModelVersionState::Approved
                | ModelVersionState::Deprecated
                | ModelVersionState::Rejected
        ) {
            return Err(moqentra_types::Error::conflict("terminal state"));
        }
        self.state = ModelVersionState::Rejected;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn deprecate(&mut self) -> Result<(), moqentra_types::Error> {
        if matches!(self.state, ModelVersionState::Rejected) {
            return Err(moqentra_types::Error::conflict("already rejected"));
        }
        self.state = ModelVersionState::Deprecated;
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }
}

/// Model family aggregate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Model {
    pub id: ModelId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub name: String,
    pub version_ids: Vec<ModelVersionId>,
    pub latest_approved: Option<ModelVersionId>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Model {
    pub fn new(
        id: ModelId,
        tenant_id: TenantId,
        project_id: ProjectId,
        name: impl Into<String>,
    ) -> Result<Self, moqentra_types::Error> {
        let name = name.into();
        if name.trim().is_empty() || name.len() > 128 {
            return Err(moqentra_types::Error::invalid_argument(
                "model name must be non-empty and at most 128 characters",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            version_ids: Vec::new(),
            latest_approved: None,
            created_at: now,
            updated_at: now,
        })
    }

    pub fn register_version(
        &mut self,
        version_id: ModelVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::conflict("version already in model"));
        }
        self.version_ids.push(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn set_latest_approved(
        &mut self,
        version_id: ModelVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if !self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::not_found("version not in model"));
        }
        self.latest_approved = Some(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }
}

/// Model artifact manifest with digest covering members.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelArtifactManifest {
    pub version: String,
    pub model_version_id: ModelVersionId,
    pub artifacts: Vec<Artifact>,
    pub signature: ModelSignature,
    pub lineage: ModelLineage,
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{DatasetVersionId, RandomIdGenerator, UserId};

    fn make_lineage() -> ModelLineage {
        let gen = RandomIdGenerator;
        ModelLineage {
            training_job_id: None,
            experiment_id: None,
            dataset_version_id: DatasetVersionId::new_v7(&gen),
            attempt_id: None,
            annotation_project_id: None,
            base_model_version_id: None,
            code_digest: "sha256:5694d08a2e53ffcae0c3103e5ad6f6076abd960eb1f8a56577040bc1028f702b"
                .to_string(),
            image_digest: "sha256:6105d6cc76af400325e94d588ce511be5bfdbb73b437dc51eca43917d7a43e3d"
                .to_string(),
            hyperparameter_digest:
                "sha256:a8aa236e33e65ccc368827e0af1497b5f655cd460b9db8ebd82ad415d59ad0f2"
                    .to_string(),
        }
    }

    #[test]
    fn model_version_lifecycle() {
        let gen = RandomIdGenerator;
        let lineage = make_lineage();
        let mut mv = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            ModelId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            lineage,
        )
        .unwrap();
        mv.artifacts.push(Artifact {
            asset_id: AssetId::new_v7(&gen),
            digest: "sha256:9372c470eeadd5ecd9c3c74c2b3cb633f8e2f2fad799250a0f70d652b6b825e4"
                .to_string(),
            size_bytes: 1024,
            media_type: "application/octet-stream".to_string(),
            scan_status: "clean".to_string(),
        });
        mv.validate().unwrap();
        mv.mark_ready().unwrap();
        mv.approve(UserId::new_v7(&gen)).unwrap();
        assert!(matches!(mv.state, ModelVersionState::Approved));
    }

    #[test]
    fn dirty_artifact_blocked() {
        let gen = RandomIdGenerator;
        let lineage = make_lineage();
        let mut mv = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            ModelId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            lineage,
        )
        .unwrap();
        mv.artifacts.push(Artifact {
            asset_id: AssetId::new_v7(&gen),
            digest: "sha256:d0c92c764c5647d43d7166efc3e7d350751de063c1388bd2e113fab4865529d3"
                .to_string(),
            size_bytes: 1,
            media_type: "application/octet-stream".to_string(),
            scan_status: "infected".to_string(),
        });
        assert!(mv.validate().is_err());
    }

    #[test]
    fn approve_requires_ready() {
        let gen = RandomIdGenerator;
        let lineage = make_lineage();
        let mut mv = ModelVersion::new(
            ModelVersionId::new_v7(&gen),
            ModelId::new_v7(&gen),
            TenantId::new_v7(&gen),
            ProjectId::new_v7(&gen),
            "v1",
            lineage,
        )
        .unwrap();
        assert!(mv.approve(UserId::new_v7(&gen)).is_err());
    }
}
