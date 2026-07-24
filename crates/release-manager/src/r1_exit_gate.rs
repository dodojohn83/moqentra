//! R1 release exit gate (Gate 0).
//!
//! R2 work must not start until R1 capabilities are tracked by real evidence,
//! accepted risks are time-boxed, and the v0.1.0 release manifest is complete.
//! This gate prevents mock/simulator results from being treated as production
//! evidence.

use moqentra_types::UtcTimestamp;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use time::{Date, Duration, Month, OffsetDateTime, PrimitiveDateTime, Time};

/// Capability state used in the R1 release tracking matrix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityState {
    Designed,
    Implemented,
    Integrated,
    Accepted,
}

impl CapabilityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            CapabilityState::Designed => "designed",
            CapabilityState::Implemented => "implemented",
            CapabilityState::Integrated => "integrated",
            CapabilityState::Accepted => "accepted",
        }
    }
}

/// A risk accepted as a blocker workaround for R1, with an owner and expiry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcceptedRisk {
    pub id: String,
    pub owner: String,
    pub expires_at: String,
}

/// R1 exit gate.
#[derive(Debug, Clone, Default)]
pub struct R1ExitGate {
    /// Capability id -> state.  Only `Accepted` capabilities can be claimed for
    /// production in the v0.1.0 release.
    pub capabilities: BTreeMap<String, CapabilityState>,
    pub release_manifest_generated: bool,
    pub schema_baseline_frozen: bool,
    pub openapi_baseline_frozen: bool,
    pub migration_baseline_frozen: bool,
    pub accepted_risks: Vec<AcceptedRisk>,
    pub unresolved_blockers: BTreeSet<String>,
}

fn parse_expiry(s: &str) -> Result<OffsetDateTime, moqentra_types::Error> {
    if let Ok(dt) = OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339) {
        return Ok(dt);
    }
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() == 3 {
        if let (Ok(year), Ok(month), Ok(day)) = (
            parts[0].parse::<i32>(),
            parts[1].parse::<u8>(),
            parts[2].parse::<u8>(),
        ) {
            if let Ok(month) = Month::try_from(month) {
                if let Ok(date) = Date::from_calendar_date(year, month, day) {
                    let end = PrimitiveDateTime::new(date, Time::MIDNIGHT)
                        .assume_utc()
                        .checked_add(Duration::days(1))
                        .ok_or_else(|| moqentra_types::Error::internal("expiry overflow"))?;
                    return Ok(end);
                }
            }
        }
    }
    Err(moqentra_types::Error::invalid_argument(
        "accepted risk expires_at must be RFC3339 or YYYY-MM-DD",
    ))
}

impl R1ExitGate {
    pub fn is_ready(&self) -> Result<(), moqentra_types::Error> {
        if !self.release_manifest_generated {
            return Err(moqentra_types::Error::unavailable(
                "v0.1.0 ReleaseManifest not generated",
            ));
        }
        if !self.schema_baseline_frozen {
            return Err(moqentra_types::Error::unavailable(
                "JSON Schema baseline not frozen",
            ));
        }
        if !self.openapi_baseline_frozen {
            return Err(moqentra_types::Error::unavailable(
                "OpenAPI baseline not frozen",
            ));
        }
        if !self.migration_baseline_frozen {
            return Err(moqentra_types::Error::unavailable(
                "migration baseline not frozen",
            ));
        }
        if !self.unresolved_blockers.is_empty() {
            return Err(moqentra_types::Error::conflict(
                "unresolved R1 blockers remain",
            ));
        }

        for (cap, state) in &self.capabilities {
            if *state != CapabilityState::Accepted {
                return Err(moqentra_types::Error::permission_denied(format!(
                    "capability {cap} is not accepted (state: {})",
                    state.as_str()
                )));
            }
        }

        let now = UtcTimestamp::now().as_offset();
        for risk in &self.accepted_risks {
            if risk.id.is_empty() || risk.owner.is_empty() || risk.expires_at.trim().is_empty() {
                return Err(moqentra_types::Error::invalid_argument(
                    "accepted risk must have id, owner and expiry",
                ));
            }
            let expiry = parse_expiry(&risk.expires_at)?;
            if now >= expiry {
                return Err(moqentra_types::Error::invalid_argument(format!(
                    "accepted risk {} has expired",
                    risk.id
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ready_gate() -> R1ExitGate {
        let mut caps = BTreeMap::new();
        caps.insert("r1-dataset".to_string(), CapabilityState::Accepted);
        caps.insert("r1-training".to_string(), CapabilityState::Accepted);
        R1ExitGate {
            capabilities: caps,
            release_manifest_generated: true,
            schema_baseline_frozen: true,
            openapi_baseline_frozen: true,
            migration_baseline_frozen: true,
            accepted_risks: vec![AcceptedRisk {
                id: "RISK-1".to_string(),
                owner: "releng".to_string(),
                expires_at: "2026-08-01".to_string(),
            }],
            unresolved_blockers: BTreeSet::new(),
        }
    }

    #[test]
    fn r1_exit_passes_when_all_accepted() {
        assert!(ready_gate().is_ready().is_ok());
    }

    #[test]
    fn r1_exit_blocks_unaccepted_capability() {
        let mut gate = ready_gate();
        gate.capabilities.insert("r1-dyun".to_string(), CapabilityState::Implemented);
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r1_exit_blocks_missing_manifest() {
        let mut gate = ready_gate();
        gate.release_manifest_generated = false;
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r1_exit_blocks_unresolved_blocker() {
        let mut gate = ready_gate();
        gate.unresolved_blockers.insert("no-real-nvidia-runner".to_string());
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r1_exit_rejects_malformed_risk() {
        let mut gate = ready_gate();
        gate.accepted_risks[0].owner = String::new();
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r1_exit_rejects_expired_risk() {
        let mut gate = ready_gate();
        gate.accepted_risks[0].expires_at = "2000-01-01".to_string();
        assert!(gate.is_ready().is_err());
    }

    #[test]
    fn r1_exit_rejects_invalid_expiry() {
        let mut gate = ready_gate();
        gate.accepted_risks[0].expires_at = "not-a-date".to_string();
        assert!(gate.is_ready().is_err());
    }
}
