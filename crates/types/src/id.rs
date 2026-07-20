//! Entity identifiers.
//!
//! All identifiers are UUID-based newtypes. They parse and serialize as
//! hyphenated UUID strings and must never be used as authorization proof.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// Generates UUIDs for identifiers.
pub trait IdGenerator {
    /// Returns a new UUIDv7 (time-sortable).
    fn v7(&self) -> Uuid;

    /// Returns a new random UUIDv4.
    fn v4(&self) -> Uuid;
}

/// Random generator backed by the standard library / uuid crate.
#[derive(Debug, Clone, Default)]
pub struct RandomIdGenerator;

impl IdGenerator for RandomIdGenerator {
    fn v7(&self) -> Uuid {
        Uuid::now_v7()
    }

    fn v4(&self) -> Uuid {
        Uuid::new_v4()
    }
}

/// Deterministic generator for tests.
#[derive(Debug, Clone)]
pub struct StaticIdGenerator {
    value: Uuid,
}

impl StaticIdGenerator {
    pub fn new(value: Uuid) -> Self {
        Self { value }
    }
}

impl IdGenerator for StaticIdGenerator {
    fn v7(&self) -> Uuid {
        self.value
    }

    fn v4(&self) -> Uuid {
        self.value
    }
}

macro_rules! define_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
        )]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Creates a new identifier from a UUIDv7 source.
            pub fn new_v7(generator: &dyn IdGenerator) -> Self {
                Self(generator.v7())
            }

            /// Creates a new identifier from a UUIDv4 source.
            pub fn new_v4(generator: &dyn IdGenerator) -> Self {
                Self(generator.v4())
            }

            /// Creates an identifier from a raw UUID.
            pub const fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Returns the underlying UUID.
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = crate::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                let uuid = Uuid::parse_str(s).map_err(|_| {
                    crate::Error::invalid_argument(format!("invalid {}: {}", stringify!($name), s))
                })?;
                Ok(Self(uuid))
            }
        }

        impl TryFrom<String> for $name {
            type Error = crate::Error;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                value.parse()
            }
        }

        impl TryFrom<&str> for $name {
            type Error = crate::Error;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                value.parse()
            }
        }
    };
}

define_id!(TenantId, "Tenant isolation boundary.");
define_id!(ProjectId, "Project within a tenant.");
define_id!(UserId, "User identity.");
define_id!(DatasetId, "Dataset within a project.");
define_id!(DatasetVersionId, "Immutable version of a dataset.");
define_id!(AssetId, "Object within a dataset version.");
define_id!(AnnotationProjectId, "Annotation project within a project.");
define_id!(AnnotationTaskId, "Annotation task assignment.");
define_id!(AnnotationId, "Individual annotation record.");
define_id!(QualityRunId, "Data quality run report.");
define_id!(AutoLabelJobId, "Automatic pre-labeling job.");
define_id!(TrainingJobId, "Training job.");
define_id!(
    AttemptId,
    "Execution attempt of a training or conversion job."
);
define_id!(ExperimentId, "Experiment grouping training jobs.");
define_id!(CheckpointId, "Training checkpoint artifact.");
define_id!(RankId, "Distributed training rank.");
define_id!(ModelVersionId, "Versioned model artifact.");
define_id!(ApplicationVersionId, "Versioned application graph.");
define_id!(
    DeploymentId,
    "Running deployment of an application version."
);
define_id!(NodeId, "Compute node agent.");
define_id!(OperationId, "Long-running operation.");
define_id!(ArtifactDigest, "Content digest of an artifact.");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_round_trip() {
        let uuid = Uuid::new_v4();
        let id = TenantId::from_uuid(uuid);
        assert_eq!(id.to_string(), uuid.to_string());
        assert_eq!(TenantId::from_str(&id.to_string()).unwrap().as_uuid(), uuid);
    }

    #[test]
    fn id_invalid() {
        assert!(TenantId::from_str("not-a-uuid").is_err());
    }

    #[test]
    fn id_generator_v7() {
        let gen = RandomIdGenerator;
        let id = TenantId::new_v7(&gen);
        assert_eq!(id.as_uuid().get_version_num(), 7);
    }

    #[test]
    fn id_serde_as_string() {
        let id = TenantId::from_uuid(Uuid::new_v4());
        let json = serde_json::to_string(&id).unwrap();
        assert!(json.contains("\""));
    }
}
