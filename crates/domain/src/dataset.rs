//! Dataset and version aggregates.

use moqentra_types::{DatasetId, DatasetVersionId, ProjectId, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// State of a dataset container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetState {
    Active,
    Archived,
}

/// State of an immutable dataset version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetVersionState {
    Draft,
    Validating,
    Published,
    Deprecated,
}

/// A dataset asset reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetRef {
    pub name: String,
    pub object_key: String,
    pub digest: String,
    pub size: u64,
    pub media_type: String,
    pub metadata: Value,
}

/// An immutable dataset version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DatasetVersion {
    pub id: DatasetVersionId,
    pub dataset_id: DatasetId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub state: DatasetVersionState,
    pub assets: Vec<AssetRef>,
    pub splits: Value,
    pub manifest_digest: Option<String>,
    pub created_at: UtcTimestamp,
    pub published_at: Option<UtcTimestamp>,
}

impl DatasetVersion {
    pub fn new(
        id: DatasetVersionId,
        dataset_id: DatasetId,
        tenant_id: TenantId,
        project_id: ProjectId,
    ) -> Self {
        Self {
            id,
            dataset_id,
            tenant_id,
            project_id,
            state: DatasetVersionState::Draft,
            assets: Vec::new(),
            splits: serde_json::Value::Null,
            manifest_digest: None,
            created_at: UtcTimestamp::now(),
            published_at: None,
        }
    }

    pub fn add_asset(&mut self, asset: AssetRef) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            DatasetVersionState::Published | DatasetVersionState::Deprecated
        ) {
            return Err(moqentra_types::Error::conflict(
                "cannot modify a published or deprecated dataset version",
            ));
        }
        self.assets.push(asset);
        Ok(())
    }

    pub fn set_splits(&mut self, splits: Value) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            DatasetVersionState::Published | DatasetVersionState::Deprecated
        ) {
            return Err(moqentra_types::Error::conflict(
                "cannot modify a published or deprecated dataset version",
            ));
        }
        self.splits = splits;
        Ok(())
    }

    pub fn publish(&mut self) -> Result<(), moqentra_types::Error> {
        if !matches!(
            self.state,
            DatasetVersionState::Draft | DatasetVersionState::Validating
        ) {
            return Err(moqentra_types::Error::conflict(
                "version is not in a publishable state",
            ));
        }
        if self.assets.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "version has no assets",
            ));
        }
        self.state = DatasetVersionState::Published;
        self.published_at = Some(UtcTimestamp::now());
        self.manifest_digest = Some(compute_manifest_digest(self)?);
        Ok(())
    }

    pub fn deprecate(&mut self) -> Result<(), moqentra_types::Error> {
        if self.state != DatasetVersionState::Published {
            return Err(moqentra_types::Error::conflict(
                "only published versions can be deprecated",
            ));
        }
        self.state = DatasetVersionState::Deprecated;
        Ok(())
    }
}

/// A mutable dataset container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dataset {
    pub id: DatasetId,
    pub tenant_id: TenantId,
    pub project_id: ProjectId,
    pub name: String,
    pub state: DatasetState,
    pub version_ids: Vec<DatasetVersionId>,
    pub latest_published_version: Option<DatasetVersionId>,
    pub labels: BTreeMap<String, String>,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

impl Dataset {
    pub fn new(
        id: DatasetId,
        tenant_id: TenantId,
        project_id: ProjectId,
        name: impl Into<String>,
    ) -> Self {
        let now = UtcTimestamp::now();
        Self {
            id,
            tenant_id,
            project_id,
            name: name.into(),
            state: DatasetState::Active,
            version_ids: Vec::new(),
            latest_published_version: None,
            labels: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_version(&mut self, version_id: DatasetVersionId) {
        self.version_ids.push(version_id);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn set_latest_published(&mut self, version_id: DatasetVersionId) {
        self.latest_published_version = Some(version_id);
        self.updated_at = UtcTimestamp::now();
    }

    pub fn archive(&mut self) {
        self.state = DatasetState::Archived;
        self.updated_at = UtcTimestamp::now();
    }
}

/// Compute a canonical digest for a version manifest.
pub fn compute_manifest_digest(version: &DatasetVersion) -> Result<String, moqentra_types::Error> {
    use sha2::{Digest, Sha256};
    let canonical = serde_json::to_string(version).map_err(|e| {
        moqentra_types::Error::internal(format!("manifest digest serialization failed: {e}"))
    })?;
    let hash = Sha256::digest(canonical.as_bytes());
    Ok(format!("sha256:{:x}", hash))
}

#[cfg(test)]
mod tests {
    use super::*;
    use moqentra_types::{DatasetId, DatasetVersionId, ProjectId, RandomIdGenerator, TenantId};

    #[test]
    fn dataset_version_lifecycle() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let dataset = DatasetId::new_v7(&gen);
        let mut version =
            DatasetVersion::new(DatasetVersionId::new_v7(&gen), dataset, tenant, project);

        version
            .add_asset(AssetRef {
                name: "train.parquet".to_string(),
                object_key: "tenants/.../train.parquet".to_string(),
                digest: "sha256:abc".to_string(),
                size: 1024,
                media_type: "application/octet-stream".to_string(),
                metadata: serde_json::json!({}),
            })
            .unwrap();

        version.set_splits(serde_json::json!({"train": 0.8, "test": 0.2})).unwrap();
        version.publish().unwrap();
        assert!(matches!(version.state, DatasetVersionState::Published));
        assert!(version.manifest_digest.is_some());

        assert!(version
            .add_asset(AssetRef {
                name: "extra.parquet".to_string(),
                object_key: "x".to_string(),
                digest: "sha256:def".to_string(),
                size: 1,
                media_type: "application/octet-stream".to_string(),
                metadata: serde_json::json!({}),
            })
            .is_err());
    }

    #[test]
    fn published_version_cannot_be_modified() {
        let gen = RandomIdGenerator;
        let tenant = TenantId::new_v7(&gen);
        let project = ProjectId::new_v7(&gen);
        let dataset = DatasetId::new_v7(&gen);
        let mut version =
            DatasetVersion::new(DatasetVersionId::new_v7(&gen), dataset, tenant, project);
        version
            .add_asset(AssetRef {
                name: "a.bin".to_string(),
                object_key: "a".to_string(),
                digest: "sha256:x".to_string(),
                size: 1,
                media_type: "application/octet-stream".to_string(),
                metadata: serde_json::json!({}),
            })
            .unwrap();
        version.publish().unwrap();
        assert!(version.set_splits(serde_json::json!({})).is_err());
    }
}
