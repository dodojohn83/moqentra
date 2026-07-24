//! Moqentra release packaging, migration, compatibility and rollback checks.

#![allow(missing_docs)]

pub mod r2_gate;

pub use r2_gate::{R2EvidenceBundle, R2PlatformTier, R2ReleaseGate};

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

impl std::str::FromStr for SupportTier {
    type Err = moqentra_types::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "certified" => Ok(Self::Certified),
            "community" => Ok(Self::Community),
            "experimental" | "preview" => Ok(Self::Experimental),
            _ => Err(moqentra_types::Error::invalid_argument(format!(
                "unknown support tier: {s}"
            ))),
        }
    }
}

/// Accepted risk with expiry and approver.  High/critical risks must have an
/// explicit, time-boxed acceptance record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RiskAcceptance {
    pub id: String,
    pub summary: String,
    pub severity: String,
    pub expires_at: String,
    pub approver: String,
}

/// Evidence bundle required for an R1 release candidate.
/// Every field must reference an externally stored report or artifact so the
/// gate cannot be bypassed by a manual boolean.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceBundle {
    pub security_scan_report_url: String,
    pub license_report_url: String,
    pub sbom_url: String,
    pub provenance_url: String,
    pub r1_e2e_report_url: String,
    pub backup_restore_report_url: String,
    pub upgrade_rollback_report_url: String,
    pub seventy_two_hour_stress_report_url: String,
    pub artifact_digests: BTreeMap<String, String>,
}

impl EvidenceBundle {
    /// Validate that every evidence URL is non-empty and parseable and that at
    /// least one artifact digest is supplied.
    fn validate(&self) -> Result<(), moqentra_types::Error> {
        let urls = [
            ("security_scan_report_url", &self.security_scan_report_url),
            ("license_report_url", &self.license_report_url),
            ("sbom_url", &self.sbom_url),
            ("provenance_url", &self.provenance_url),
            ("r1_e2e_report_url", &self.r1_e2e_report_url),
            ("backup_restore_report_url", &self.backup_restore_report_url),
            (
                "upgrade_rollback_report_url",
                &self.upgrade_rollback_report_url,
            ),
            (
                "seventy_two_hour_stress_report_url",
                &self.seventy_two_hour_stress_report_url,
            ),
        ];
        for (name, url) in &urls {
            if url.trim().is_empty() {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "evidence {name} is missing"
                )));
            }
            if url::Url::parse(url).is_err() {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "evidence {name} is not a valid URL: {url}"
                )));
            }
        }
        if self.artifact_digests.is_empty() {
            return Err(moqentra_types::Error::invalid_argument(
                "at least one artifact digest must be supplied",
            ));
        }
        for (name, digest) in &self.artifact_digests {
            if !digest.starts_with("sha256:") || digest.len() < 8 {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "artifact {name} digest must be a sha256 reference"
                )));
            }
        }
        Ok(())
    }
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
    pub r1_e2e_complete: bool,
    pub migrations_verified: bool,
    pub known_risks_accepted: BTreeSet<String>,
    pub unresolved_blockers: BTreeSet<String>,
    pub accepted_high_critical_risks: Vec<RiskAcceptance>,
    pub evidence: Option<EvidenceBundle>,
    pub ga_claimed_platforms: BTreeSet<String>,
    pub manifest: Option<ReleaseManifest>,
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
        if !self.r1_e2e_complete {
            return Err(moqentra_types::Error::unavailable(
                "R1 golden E2E not complete",
            ));
        }
        if !self.migrations_verified {
            return Err(moqentra_types::Error::unavailable(
                "migrations not verified",
            ));
        }

        for risk in &self.accepted_high_critical_risks {
            let trimmed = risk.severity.trim().to_ascii_lowercase();
            if (trimmed == "high" || trimmed == "critical") && risk.expires_at.trim().is_empty() {
                return Err(moqentra_types::Error::permission_denied(format!(
                    "high/critical risk {} must have an expiry",
                    risk.id
                )));
            }
            if risk.id.is_empty() || risk.approver.is_empty() {
                return Err(moqentra_types::Error::permission_denied(
                    "risk acceptance must have id and approver",
                ));
            }
        }

        if !self.unresolved_blockers.is_empty() {
            return Err(moqentra_types::Error::conflict(
                "unresolved blockers remain",
            ));
        }

        let evidence = self
            .evidence
            .as_ref()
            .ok_or_else(|| moqentra_types::Error::invalid_argument("evidence bundle missing"))?;
        evidence.validate()?;

        if let Some(manifest) = &self.manifest {
            for (name, spec) in &manifest.artifacts {
                if let Some(evidence_digest) = evidence.artifact_digests.get(name) {
                    if &spec.digest != evidence_digest {
                        return Err(moqentra_types::Error::conflict(format!(
                            "artifact {name} digest mismatch between manifest and evidence"
                        )));
                    }
                }
            }
        }

        // R1 GA claims must not include non-R1-ready platforms.
        let prohibited = ["amd", "ascend", "multi-node", "zero-downtime"];
        for claim in &self.ga_claimed_platforms {
            let lower = claim.to_ascii_lowercase();
            if prohibited.iter().any(|p| lower.contains(p)) {
                return Err(moqentra_types::Error::permission_denied(format!(
                    "platform {claim} is not allowed in R1 GA claims"
                )));
            }
            if lower.contains("rtx 3090") && !lower.contains("preview") {
                return Err(moqentra_types::Error::permission_denied(
                    "RTX 3090 features must be marked preview in R1",
                ));
            }
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
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use std::str::FromStr;

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

    fn minimal_evidence() -> EvidenceBundle {
        let mut digests = BTreeMap::new();
        digests.insert("control-plane".to_string(), "sha256:abcdef".to_string());
        EvidenceBundle {
            security_scan_report_url: "https://example.com/security".to_string(),
            license_report_url: "https://example.com/license".to_string(),
            sbom_url: "https://example.com/sbom".to_string(),
            provenance_url: "https://example.com/provenance".to_string(),
            r1_e2e_report_url: "https://example.com/e2e".to_string(),
            backup_restore_report_url: "https://example.com/backup".to_string(),
            upgrade_rollback_report_url: "https://example.com/upgrade".to_string(),
            seventy_two_hour_stress_report_url: "https://example.com/stress".to_string(),
            artifact_digests: digests,
        }
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
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        gate.evidence = Some(minimal_evidence());
        assert!(gate.is_ready().is_ok());
    }

    #[test]
    fn release_gate_rejects_missing_evidence() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        // Missing evidence bundle.
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn release_gate_blocks_invalid_artifact_digest() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        let mut evidence = minimal_evidence();
        evidence.artifact_digests.insert("web".to_string(), "bad-digest".to_string());
        gate.evidence = Some(evidence);
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn release_gate_blocks_ga_claims_for_prohibited_platforms() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        gate.evidence = Some(minimal_evidence());
        gate.ga_claimed_platforms.insert("Ascend NPU".to_string());
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn release_gate_blocks_rtx_3090_without_preview() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        gate.evidence = Some(minimal_evidence());
        gate.ga_claimed_platforms.insert("NVIDIA GeForce RTX 3090".to_string());
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn release_gate_allows_rtx_3090_preview() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        gate.evidence = Some(minimal_evidence());
        gate.ga_claimed_platforms
            .insert("NVIDIA GeForce RTX 3090 (preview)".to_string());
        assert!(gate.is_ready().is_ok());
    }

    #[test]
    fn release_gate_detects_digest_mismatch() {
        let mut gate = ReleaseGate::default();
        gate.security_scan_clean = true;
        gate.license_check_clean = true;
        gate.sbom_attached = true;
        gate.provenance_attached = true;
        gate.signatures_valid = true;
        gate.seventy_two_hour_burn = true;
        gate.disaster_recovery_drill = true;
        gate.rollback_tested = true;
        gate.r1_e2e_complete = true;
        gate.migrations_verified = true;
        gate.evidence = Some(minimal_evidence());

        let mut manifest = ReleaseManifest {
            version: "0.1.0".to_string(),
            commit: "abc".to_string(),
            artifacts: BTreeMap::new(),
            sbom_reference: "sbom.json".to_string(),
            provenance_reference: "provenance.json".to_string(),
            signatures: BTreeMap::new(),
        };
        manifest.artifacts.insert(
            "control-plane".to_string(),
            ArtifactSpec {
                image: "moqentra/control-plane".to_string(),
                digest: "sha256:other".to_string(),
                architectures: BTreeSet::new(),
                platform_tiers: BTreeMap::new(),
            },
        );
        gate.manifest = Some(manifest);
        assert!(gate.is_ready().is_err());
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

    #[test]
    fn support_tier_parses_preview_as_experimental() {
        assert_eq!(
            SupportTier::from_str("preview").unwrap(),
            SupportTier::Experimental
        );
        assert_eq!(
            SupportTier::from_str("Certified").unwrap(),
            SupportTier::Certified
        );
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
}
