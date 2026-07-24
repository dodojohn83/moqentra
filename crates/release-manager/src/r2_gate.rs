//! R2 release gate and evidence checks for v0.2.0.
//!
//! This module extends the generic `ReleaseGate` with R2-specific evidence
//! requirements: multi-node NVIDIA DDP, conversion support tiers, cross-tenant
//! penetration, control-plane/leader HA, RPO/RTO, 72h burn, upgrade/rollback,
//! and hardware platform support matrix.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Per-platform support tier in the R2 release matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum R2PlatformTier {
    Verified,
    Preview,
    CompileOnly,
    Unsupported,
}

impl R2PlatformTier {
    pub fn as_str(&self) -> &'static str {
        match self {
            R2PlatformTier::Verified => "verified",
            R2PlatformTier::Preview => "preview",
            R2PlatformTier::CompileOnly => "compile-only",
            R2PlatformTier::Unsupported => "unsupported",
        }
    }

    pub fn can_be_ga_claimed(&self) -> bool {
        matches!(self, R2PlatformTier::Verified)
    }
}

/// Evidence bundle required for an R2 release candidate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct R2EvidenceBundle {
    pub nvidia_multinode_ddp_report_url: String,
    pub conversion_onnx_report_url: String,
    pub conversion_tensorrt_report_url: String,
    pub conversion_openvino_report_url: String,
    pub cross_tenant_penetration_report_url: String,
    pub control_plane_ha_report_url: String,
    pub rpo_rto_report_url: String,
    pub upgrade_rollback_report_url: String,
    pub seventy_two_hour_burn_report_url: String,
    pub artifact_digests: BTreeMap<String, String>,
}

impl R2EvidenceBundle {
    fn validate(&self) -> Result<(), moqentra_types::Error> {
        let urls = [
            (
                "nvidia_multinode_ddp_report_url",
                &self.nvidia_multinode_ddp_report_url,
            ),
            (
                "conversion_onnx_report_url",
                &self.conversion_onnx_report_url,
            ),
            (
                "conversion_tensorrt_report_url",
                &self.conversion_tensorrt_report_url,
            ),
            (
                "conversion_openvino_report_url",
                &self.conversion_openvino_report_url,
            ),
            (
                "cross_tenant_penetration_report_url",
                &self.cross_tenant_penetration_report_url,
            ),
            (
                "control_plane_ha_report_url",
                &self.control_plane_ha_report_url,
            ),
            ("rpo_rto_report_url", &self.rpo_rto_report_url),
            (
                "upgrade_rollback_report_url",
                &self.upgrade_rollback_report_url,
            ),
            (
                "seventy_two_hour_burn_report_url",
                &self.seventy_two_hour_burn_report_url,
            ),
        ];
        for (name, url) in &urls {
            if url.trim().is_empty() {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "r2 evidence {name} is missing"
                )));
            }
            if url::Url::parse(url).is_err() {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "r2 evidence {name} is not a valid URL: {url}"
                )));
            }
        }
        for (name, digest) in &self.artifact_digests {
            if !digest.starts_with("sha256:") || digest.len() < 8 {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "r2 artifact {name} digest must be a sha256 reference"
                )));
            }
        }
        Ok(())
    }
}

/// R2 release gate.
#[derive(Debug, Clone, Default)]
pub struct R2ReleaseGate {
    pub security_scan_clean: bool,
    pub license_check_clean: bool,
    pub sbom_attached: bool,
    pub provenance_attached: bool,
    pub signatures_valid: bool,
    pub migrations_verified: bool,
    pub evidence: Option<R2EvidenceBundle>,
    pub platform_tiers: BTreeMap<String, R2PlatformTier>,
    pub unresolved_blockers: BTreeSet<String>,
}

impl R2ReleaseGate {
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
        if !self.migrations_verified {
            return Err(moqentra_types::Error::unavailable(
                "migrations not verified",
            ));
        }
        if !self.unresolved_blockers.is_empty() {
            return Err(moqentra_types::Error::conflict(
                "unresolved blockers remain",
            ));
        }

        let evidence = self
            .evidence
            .as_ref()
            .ok_or_else(|| moqentra_types::Error::invalid_argument("r2 evidence bundle missing"))?;
        evidence.validate()?;

        // Hard gate: NVIDIA multi-node DDP must be verified for v0.2.0.
        match self.platform_tiers.get("nvidia-multinode") {
            Some(R2PlatformTier::Verified) => {}
            Some(tier) => {
                return Err(moqentra_types::Error::permission_denied(format!(
                    "NVIDIA multi-node DDP must be verified for R2 GA, got {}",
                    tier.as_str()
                )));
            }
            None => {
                return Err(moqentra_types::Error::invalid_argument(
                    "NVIDIA multi-node DDP tier missing from release matrix",
                ));
            }
        }

        // Cross-tenant penetration and HA are hard gates.
        for gate in [
            "cross-tenant-penetration",
            "control-plane-ha",
            "rpo-rto",
            "upgrade-rollback",
            "72h-burn",
        ] {
            if !evidence_has_gate(evidence, gate) {
                return Err(moqentra_types::Error::unavailable(format!(
                    "{gate} evidence missing"
                )));
            }
        }

        // AMD/Ascend/OM/RKNN/Sophon may not be GA-claimed without real runner evidence.
        for (platform, tier) in &self.platform_tiers {
            if tier.can_be_ga_claimed() {
                let lower = platform.to_ascii_lowercase();
                if lower.starts_with("amd")
                    || lower.starts_with("ascend")
                    || lower.contains("om")
                    || lower.contains("rknn")
                    || lower.contains("sophon")
                {
                    return Err(moqentra_types::Error::permission_denied(format!(
                        "platform {platform} cannot be GA-claimed without real hardware evidence"
                    )));
                }
            }
        }

        Ok(())
    }
}

fn evidence_has_gate(evidence: &R2EvidenceBundle, gate: &str) -> bool {
    let url = match gate {
        "cross-tenant-penetration" => &evidence.cross_tenant_penetration_report_url,
        "control-plane-ha" => &evidence.control_plane_ha_report_url,
        "rpo-rto" => &evidence.rpo_rto_report_url,
        "upgrade-rollback" => &evidence.upgrade_rollback_report_url,
        "72h-burn" => &evidence.seventy_two_hour_burn_report_url,
        _ => return false,
    };
    !url.trim().is_empty() && url::Url::parse(url).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> R2EvidenceBundle {
        R2EvidenceBundle {
            nvidia_multinode_ddp_report_url: "https://example.com/ddp".to_string(),
            conversion_onnx_report_url: "https://example.com/onnx".to_string(),
            conversion_tensorrt_report_url: "https://example.com/trt".to_string(),
            conversion_openvino_report_url: "https://example.com/ov".to_string(),
            cross_tenant_penetration_report_url: "https://example.com/pen".to_string(),
            control_plane_ha_report_url: "https://example.com/ha".to_string(),
            rpo_rto_report_url: "https://example.com/rpo".to_string(),
            upgrade_rollback_report_url: "https://example.com/ub".to_string(),
            seventy_two_hour_burn_report_url: "https://example.com/burn".to_string(),
            artifact_digests: BTreeMap::new(),
        }
    }

    fn ready_gate() -> R2ReleaseGate {
        let mut tiers = BTreeMap::new();
        tiers.insert("nvidia-multinode".to_string(), R2PlatformTier::Verified);
        tiers.insert("amd-mi300".to_string(), R2PlatformTier::CompileOnly);
        R2ReleaseGate {
            security_scan_clean: true,
            license_check_clean: true,
            sbom_attached: true,
            provenance_attached: true,
            signatures_valid: true,
            migrations_verified: true,
            evidence: Some(evidence()),
            platform_tiers: tiers,
            unresolved_blockers: BTreeSet::new(),
        }
    }

    #[test]
    fn r2_gate_passes_with_verified_nvidia() {
        assert!(ready_gate().is_ready().is_ok());
    }

    #[test]
    fn r2_gate_blocks_missing_nvidia() {
        let mut gate = ready_gate();
        gate.platform_tiers.remove("nvidia-multinode");
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r2_gate_blocks_preview_nvidia() {
        let mut gate = ready_gate();
        gate.platform_tiers
            .insert("nvidia-multinode".to_string(), R2PlatformTier::Preview);
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r2_gate_blocks_ga_claim_for_amd() {
        let mut gate = ready_gate();
        gate.platform_tiers.insert("amd-mi300".to_string(), R2PlatformTier::Verified);
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r2_gate_blocks_unresolved_blocker() {
        let mut gate = ready_gate();
        gate.unresolved_blockers.insert("no-real-nvidia-runner".to_string());
        assert!(gate.is_ready().is_err());
    }
}
