//! Dataset and version aggregates.

use moqentra_types::{DatasetId, DatasetVersionId, ProjectId, TenantId, UtcTimestamp};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::str::FromStr;

/// State of a dataset container.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetState {
    Active,
    Archived,
}

impl FromStr for DatasetState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Active" => Ok(Self::Active),
            "Archived" => Ok(Self::Archived),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown dataset state: {s}"
            ))),
        }
    }
}

/// State of an immutable dataset version.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DatasetVersionState {
    Draft,
    Validating,
    Published,
    Deprecated,
}

impl FromStr for DatasetVersionState {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Draft" => Ok(Self::Draft),
            "Validating" => Ok(Self::Validating),
            "Published" => Ok(Self::Published),
            "Deprecated" => Ok(Self::Deprecated),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown dataset version state: {s}"
            ))),
        }
    }
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

impl AssetRef {
    fn validate(&self) -> Result<(), moqentra_types::Error> {
        if self.name.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "asset name is empty",
            ));
        }
        if self.object_key.is_empty() || self.object_key.contains('\0') {
            return Err(moqentra_types::Error::invalid_argument(
                "asset object_key is invalid",
            ));
        }
        if self.size == 0 {
            return Err(moqentra_types::Error::invalid_argument(
                "asset size must be greater than zero",
            ));
        }
        if self.media_type.trim().is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "asset media_type is empty",
            ));
        }
        if !moqentra_types::valid_content_digest(&self.digest) {
            return Err(moqentra_types::Error::invalid_argument(
                "asset digest must be a valid content digest",
            ));
        }
        Ok(())
    }
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
        if self.assets.iter().any(|a| a.name == asset.name) {
            return Err(moqentra_types::Error::conflict(
                "asset name already exists in version",
            ));
        }
        if self.assets.iter().any(|a| a.object_key == asset.object_key) {
            return Err(moqentra_types::Error::conflict(
                "object key already exists in version",
            ));
        }
        asset.validate()?;
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

    /// Generate deterministic train/val/test splits using a fixed seed.
    ///
    /// The fractions must sum to 1.0 within a small epsilon. Each asset is
    /// assigned to exactly one split by a Fisher-Yates shuffle driven by a
    /// SplitMix64 PRNG seeded with `seed`.
    pub fn generate_splits(
        &mut self,
        seed: u64,
        train: f64,
        val: f64,
        test: f64,
    ) -> Result<(), moqentra_types::Error> {
        if matches!(
            self.state,
            DatasetVersionState::Published | DatasetVersionState::Deprecated
        ) {
            return Err(moqentra_types::Error::conflict(
                "cannot modify a published or deprecated dataset version",
            ));
        }
        if self.assets.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "version has no assets to split",
            ));
        }
        let sum = train + val + test;
        if (sum - 1.0).abs() > 1e-9 || train < 0.0 || val < 0.0 || test < 0.0 {
            return Err(moqentra_types::Error::invalid_argument(
                "train/val/test fractions must be non-negative and sum to 1.0",
            ));
        }

        let mut names: Vec<String> = self.assets.iter().map(|a| a.name.clone()).collect();

        // Deterministic Fisher-Yates using SplitMix64.
        let mut rng = seed;
        for i in (1..names.len()).rev() {
            rng = splitmix64(rng);
            let j = (rng % (i as u64 + 1)) as usize;
            names.swap(i, j);
        }

        let n = names.len();
        let train_end = ((train * n as f64).round() as usize).clamp(0, n);
        let val_end = train_end + ((val * n as f64).round() as usize).clamp(0, n - train_end);

        let (train_names, rest) = names.split_at(train_end);
        let (val_names, test_names) = rest.split_at(val_end - train_end);

        self.splits = serde_json::json!({
            "seed": seed,
            "train": train_names,
            "val": val_names,
            "test": test_names,
            "train_fraction": train,
            "val_fraction": val,
            "test_fraction": test,
        });
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
        let digest = compute_manifest_digest(self)?;
        self.state = DatasetVersionState::Published;
        self.published_at = Some(UtcTimestamp::now());
        self.manifest_digest = Some(digest);
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
    ) -> Result<Self, moqentra_types::Error> {
        let name = name.into();
        if name.trim().is_empty() || name.len() > 128 {
            return Err(moqentra_types::Error::invalid_argument(
                "dataset name must be non-empty and at most 128 characters",
            ));
        }
        let now = UtcTimestamp::now();
        Ok(Self {
            id,
            tenant_id,
            project_id,
            name,
            state: DatasetState::Active,
            version_ids: Vec::new(),
            latest_published_version: None,
            labels: BTreeMap::new(),
            created_at: now,
            updated_at: now,
        })
    }

    pub fn add_version(
        &mut self,
        version_id: DatasetVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::conflict(
                "version already in dataset",
            ));
        }
        self.version_ids.push(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn set_latest_published(
        &mut self,
        version_id: DatasetVersionId,
    ) -> Result<(), moqentra_types::Error> {
        if !self.version_ids.contains(&version_id) {
            return Err(moqentra_types::Error::not_found("version not in dataset"));
        }
        self.latest_published_version = Some(version_id);
        self.updated_at = UtcTimestamp::now();
        Ok(())
    }

    pub fn archive(&mut self) {
        self.state = DatasetState::Archived;
        self.updated_at = UtcTimestamp::now();
    }
}

/// Compute a canonical digest for a version manifest.
///
/// The digest is taken over the immutable contents (ids, assets and splits) so
/// that it remains stable when the version state or publish timestamp changes.
fn splitmix64(state: u64) -> u64 {
    let mut z = state.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

pub fn compute_manifest_digest(version: &DatasetVersion) -> Result<String, moqentra_types::Error> {
    use serde::Serialize;
    use sha2::{Digest, Sha256};

    #[derive(Serialize)]
    struct CanonicalManifest<'a> {
        id: &'a moqentra_types::DatasetVersionId,
        dataset_id: &'a moqentra_types::DatasetId,
        tenant_id: &'a moqentra_types::TenantId,
        project_id: &'a moqentra_types::ProjectId,
        assets: &'a Vec<AssetRef>,
        splits: &'a serde_json::Value,
    }

    let canonical = CanonicalManifest {
        id: &version.id,
        dataset_id: &version.dataset_id,
        tenant_id: &version.tenant_id,
        project_id: &version.project_id,
        assets: &version.assets,
        splits: &version.splits,
    };
    let canonical_json = serde_json::to_string(&canonical).map_err(|e| {
        moqentra_types::Error::internal(format!("manifest digest serialization failed: {e}"))
    })?;
    let hash = Sha256::digest(canonical_json.as_bytes());
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
                digest: "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
                    .to_string(),
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
                digest: "sha256:cb8379ac2098aa165029e3938a51da0bcecfc008fd6795f401178647f96c5b34"
                    .to_string(),
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
                digest: "sha256:2d711642b726b04401627ca9fbac32f5c8530fb1903cc4db02258717921a4881"
                    .to_string(),
                size: 1,
                media_type: "application/octet-stream".to_string(),
                metadata: serde_json::json!({}),
            })
            .unwrap();
        version.publish().unwrap();
        assert!(version.set_splits(serde_json::json!({})).is_err());
    }
}
