//! Moqentra release packaging, migration, compatibility and rollback checks.

#![allow(missing_docs)]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Compatibility contract between two adjacent versions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Compatibility {
    pub old_version: String,
    pub new_version: String,
    pub control_plane_compatible: bool,
    pub worker_compatible: bool,
    pub agent_compatible: bool,
    pub schema_add_only: bool,
    pub sdk_compatible: bool,
}

impl Compatibility {
    pub fn allows_rolling_upgrade(&self) -> bool {
        self.control_plane_compatible
            && self.worker_compatible
            && self.agent_compatible
            && self.schema_add_only
            && self.sdk_compatible
    }
}

/// Release manifest with artifacts, SBOM/provenance and signatures.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub version: String,
    pub commit: String,
    pub artifacts: BTreeMap<String, ArtifactSpec>,
    pub sbom_reference: String,
    pub provenance_reference: String,
    pub signatures: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactSpec {
    pub image: String,
    pub digest: String,
    pub architectures: BTreeSet<String>,
    pub platform_tiers: BTreeMap<String, SupportTier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SupportTier {
    Certified,
    Community,
    Experimental,
}

/// Release gate checks.
#[derive(Debug, Clone, Default)]
pub struct ReleaseGate {
    pub security_scan_clean: bool,
    pub license_check_clean: bool,
    pub sbom_attached: bool,
    pub provenance_attached: bool,
    pub signatures_valid: bool,
    pub seventy_two_hour_burn: bool,
    pub disaster_recovery_drill: bool,
    pub rollback_tested: bool,
    pub known_risks_accepted: BTreeSet<String>,
    pub unresolved_blockers: BTreeSet<String>,
}

impl ReleaseGate {
    pub fn is_ready(&self) -> Result<(), moqentra_types::Error> {
        if !self.security_scan_clean {
            return Err(moqentra_types::Error::permission_denied(
                "security scan not clean",
            ));
        }
        if !self.license_check_clean {
            return Err(moqentra_types::Error::permission_denied(
                "license check not clean",
            ));
        }
        if !self.sbom_attached || !self.provenance_attached || !self.signatures_valid {
            return Err(moqentra_types::Error::permission_denied(
                "release artifacts not signed/provenanced",
            ));
        }
        if !self.seventy_two_hour_burn {
            return Err(moqentra_types::Error::unavailable(
                "72h burn-in not complete",
            ));
        }
        if !self.disaster_recovery_drill {
            return Err(moqentra_types::Error::unavailable(
                "disaster recovery drill not complete",
            ));
        }
        if !self.rollback_tested {
            return Err(moqentra_types::Error::unavailable("rollback not tested"));
        }
        if !self.unresolved_blockers.is_empty() {
            return Err(moqentra_types::Error::conflict(
                "unresolved blockers remain",
            ));
        }
        Ok(())
    }
}

/// Helm/Compose release values validator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeploymentValues {
    pub external_db: bool,
    pub external_s3: bool,
    pub external_oidc: bool,
    pub network_policy_enabled: bool,
    pub pod_disruption_budget_enabled: bool,
    pub resource_limits: bool,
    pub upgrade_hooks: bool,
}

impl DeploymentValues {
    pub fn validate(&self) -> Result<(), moqentra_types::Error> {
        if !(self.external_db && self.external_s3 && self.external_oidc) {
            return Err(moqentra_types::Error::invalid_argument(
                "production requires external db/s3/oidc",
            ));
        }
        if !self.network_policy_enabled {
            return Err(moqentra_types::Error::permission_denied(
                "network policy must be enabled",
            ));
        }
        if !self.resource_limits {
            return Err(moqentra_types::Error::invalid_argument(
                "resource limits must be set",
            ));
        }
        if !self.pod_disruption_budget_enabled {
            return Err(moqentra_types::Error::invalid_argument(
                "pod disruption budget must be enabled",
            ));
        }
        if !self.upgrade_hooks {
            return Err(moqentra_types::Error::invalid_argument(
                "upgrade hooks must be enabled",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compatibility_allows_rolling_upgrade() {
        let c = Compatibility {
            old_version: "0.1.0".to_string(),
            new_version: "0.2.0".to_string(),
            control_plane_compatible: true,
            worker_compatible: true,
            agent_compatible: true,
            schema_add_only: true,
            sdk_compatible: true,
        };
        assert!(c.allows_rolling_upgrade());
    }

    #[test]
    fn release_gate_blocks_missing_checks() {
        let mut gate = ReleaseGate::default();
        assert!(gate.is_ready().is_err());
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        assert!(gate.is_ready().is_ok());
    }

    #[test]
    fn deployment_values_require_external_services() {
        let v = DeploymentValues {
            external_db: true,
            external_s3: true,
            external_oidc: true,
            network_policy_enabled: true,
            pod_disruption_budget_enabled: true,
            resource_limits: true,
            upgrade_hooks: true,
        };
        assert!(v.validate().is_ok());

        let mut bad = v.clone();
        bad.external_s3 = false;
        assert!(bad.validate().is_err());

        bad = v;
        bad.network_policy_enabled = false;
        assert!(bad.validate().is_err());
    }

    #[test]
    fn release_manifest_carries_sbom_and_signature() {
        let mut manifest = ReleaseManifest {
            version: "0.1.0".to_string(),
            commit: "abc".to_string(),
            artifacts: BTreeMap::new(),
            sbom_reference: "sbom.json".to_string(),
            provenance_reference: "provenance.json".to_string(),
            signatures: BTreeMap::new(),
        };
        manifest.signatures.insert("control-plane".to_string(), "sig".to_string());
        assert!(manifest.signatures.contains_key("control-plane"));
    }
}
